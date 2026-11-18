import { chooseOption } from '../e2e/select'
import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import { randomUUID } from 'node:crypto'
import {
  AccountList,
  ApiError,
  GiveBpResult,
  StaffSession,
} from '../src/api/generated/validation/schemas'

async function signIn(page: Page, user = 'CheckAdmin') {
  await page.goto('/')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('GateCheck12345')
  const signingIn = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  StaffSession.parse(await (await signingIn).json())
  await expect(page.getByRole('button', { name: 'Sign out', exact: true })).toBeVisible()
}

test('production pages and lazy assets work through the released nginx and real staff API', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('response', (response) => {
    const path = new URL(response.url()).pathname
    if (path.startsWith('/assets/') && !response.ok())
      errors.push(`Asset failed: ${response.status()} ${response.url()}`)
    if (
      path.startsWith('/panel/api/') &&
      response.status() >= 400 &&
      !(path.endsWith('/session') && response.status() === 401)
    )
      errors.push(`API failed: ${response.status()} ${path}`)
  })
  await signIn(page)
  await expect(page.locator('.page-footer')).toContainText(/Server v\S+ · Dashboard /)
  for (const [path, title] of [
    ['/', 'Overview'],
    ['/accounts', 'Accounts'],
    ['/invites', 'Invites & codes'],
    ['/news', 'News & maintenance'],
    ['/moderation', 'Moderation'],
    ['/audit', 'Audit log'],
    ['/logs', 'Server logs'],
    ['/backups', 'Backups'],
    ['/settings', 'Settings'],
  ]) {
    await page.goto(path)
    await expect(page.getByRole('heading', { level: 1, name: title, exact: true })).toBeVisible()
    await expect(page.getByText(/Sample data.*not connected/)).toHaveCount(0)
    await expect(page.getByText(/Loading workspace/)).toHaveCount(0)
  }
  expect(errors).toEqual([])
  const schema = await page.request.get('/panel/openapi.json')
  expect(schema.ok()).toBe(true)
  expect(await schema.json()).toEqual(
    JSON.parse(await (await import('node:fs/promises')).readFile('api/openapi.json', 'utf8')),
  )
  for (const path of [
    '/api/admin/v1/status',
    '/panel/',
    '/health',
    '/panel/api/v1/backups/download',
  ])
    expect((await page.request.get(path)).status(), path).toBe(404)
})

test('a real BP change survives a lost response and retry without paying twice', async ({
  page,
}) => {
  await signIn(page)
  const before = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  ).accounts.find((account) => account.login === 'CheckModerator')!
  await page.getByRole('link', { name: 'Accounts', exact: true }).click()
  await page.getByRole('button', { name: 'Give BP to CheckModerator', exact: true }).click()
  const dialog = page.getByRole('dialog')
  const ids: string[] = []
  let result: ReturnType<typeof GiveBpResult.parse> | undefined
  await page.route('**/panel/api/v1/give-bp', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    result = GiveBpResult.parse(await response.json())
    if (ids.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await dialog.getByLabel('BP amount').fill('13')
  await dialog.getByRole('button', { name: 'Add BP', exact: true }).click()
  await expect(dialog.getByText(/could not confirm/i)).toBeVisible()
  await dialog.getByRole('button', { name: 'Retry same grant', exact: true }).click()
  await expect(page.getByRole('status')).toContainText('BP added')
  expect(ids).toHaveLength(2)
  expect(ids[1]).toBe(ids[0])
  expect(result?.bp).toBe(before.bp + 13)
  const accounts = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  )
  expect(accounts.accounts.find((a) => a.login === 'CheckModerator')?.bp).toBe(before.bp + 13)
})

test('moderator restrictions and expired sessions are enforced by the released server', async ({
  page,
}) => {
  await signIn(page, 'CheckModerator')
  const session = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  const accounts = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  )
  const admin = accounts.accounts.find((account) => account.login === 'CheckAdmin')!
  const denied = await page.request.post('/panel/api/v1/give-bp', {
    headers: {
      Origin: new URL(page.url()).origin,
      'X-CSRF-Token': session.csrf_token,
      'Idempotency-Key': randomUUID(),
    },
    data: { user_id: admin.user_id, amount: 1 },
  })
  expect(denied.status()).toBe(403)
  expect(ApiError.parse(await denied.json()).code).toBe('forbidden')
  await page.goto('/settings')
  await expect(page.getByText('Only administrators can change server settings.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Edit', exact: true }).first()).toBeDisabled()
  await page.context().clearCookies()
  await page.reload()
  await expect(page.getByRole('heading', { name: 'Sign in to your server' })).toBeVisible()
})

test('an open form survives an update notice and a missing lazy asset has a reload action', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('link', { name: 'Accounts', exact: true }).click()
  await page.getByRole('button', { name: 'Give BP to CheckModerator', exact: true }).click()
  const dialog = page.getByRole('dialog')
  await dialog.getByLabel('BP amount').fill('77')
  await page.route('**/assets/dashboard-version.json?*', (route) =>
    route.fulfill({ json: { commit: 'd'.repeat(40) } }),
  )
  await page.evaluate(() => window.dispatchEvent(new Event('focus')))
  await expect(page.getByText('An update is available.')).toBeVisible()
  await expect(dialog.getByLabel('BP amount')).toHaveValue('77')
  await page.screenshot({ path: '../logs/dashboard-update-notice.png' })
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  // Earlier pages may have preloaded the module already. Start a document with that real asset unavailable.
  await page.route('**/assets/ConnectedInvites-*.js', (route) => route.abort())
  await page.goto('/invites')
  await expect(page.getByText('This page needs the latest dashboard files.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Reload dashboard' }).first()).toBeVisible()
})

test('account deep links reload through nginx while unrelated routes stay closed', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('link', { name: 'Accounts', exact: true }).click()
  await page.getByRole('link', { name: 'View account CheckAdmin', exact: true }).click()
  const url = page.url()
  await expect(
    page.getByRole('heading', { level: 1, name: 'CheckAdmin', exact: true }),
  ).toBeVisible()
  const response = await page.reload()
  expect(response?.status()).toBe(200)
  await expect(page).toHaveURL(url)
  await expect(page.getByRole('region', { name: 'Players', exact: true })).toBeVisible()
  for (const path of ['/accounts/1/extra', '/accounts/settings', '/accounts/1/admin'])
    expect((await page.request.get(path)).status(), path).toBe(404)
})

test('a saved dark theme and grouped settings work under the production security policy', async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: 'light' })
  await page.goto('/')
  await page.getByRole('button', { name: 'Switch to dark mode', exact: true }).click()
  await signIn(page)
  await page.getByRole('link', { name: 'Settings', exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await expect(page.getByLabel('Color theme', { exact: true })).toHaveAttribute(
    'data-value',
    'dark',
  )
  await chooseOption(page.getByLabel('Setting group', { exact: true }), 'Duels')
  await expect(page.getByTestId('setting-start-lp')).toBeVisible()
  await expect(page.getByTestId('setting-server-name')).toHaveCount(0)
  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await expect(page.getByTestId('setting-server-name')).toBeVisible()
})
