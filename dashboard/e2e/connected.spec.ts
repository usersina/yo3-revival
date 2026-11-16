import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import {
  AccountList,
  BpCodeList,
  InviteList,
  CreateInviteResult,
  CreateBpCodeResult,
  RemoveBpCodeResult,
  ApiError,
  GiveBpResult,
  StaffSession,
} from '../src/api/generated/validation/schemas'

async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect(StaffSession.safeParse(await (await response).json()).success).toBe(true)
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}

test('real sign-in and account responses follow the generated contract; moderator cannot grant', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  const response = await page.request.get('/panel/api/v1/accounts')
  expect(AccountList.safeParse(await response.json()).success).toBe(true)
  await expect(page.getByRole('button', { name: /Give BP/ }).first()).toBeDisabled()
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible()
})

test('lost response and a page reload keep the same grant ID; no background retry can pay twice', async ({
  page,
}) => {
  await signIn(page)
  const before = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  ).accounts
  const target = before.find((a) => a.login === 'TestPlayer')!
  const ids: string[] = []
  await page.route('**/panel/api/v1/give-bp', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    if (ids.length === 2) {
      const refused = await route.fetch({
        headers: { ...route.request().headers(), 'x-csrf-token': 'stale-token' },
      })
      expect(refused.status()).toBe(403)
      expect(ApiError.parse(await refused.json()).code).toBe('csrf_invalid')
      return route.fulfill({ response: refused })
    }
    const response = await route.fetch()
    const body = await response.json()
    expect(
      GiveBpResult.safeParse(body).success,
      JSON.stringify({ status: response.status(), body }),
    ).toBe(true)
    if (ids.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await page.getByRole('button', { name: 'Give BP to TestPlayer' }).click()
  await page.getByLabel('BP amount').fill('123')
  await page.getByRole('button', { name: 'Add BP', exact: true }).click()
  await expect(page.getByText(/could not confirm/i)).toBeVisible()
  expect(ids).toHaveLength(1)
  await page.reload()
  await expect(page.getByText('Unconfirmed grant')).toBeVisible()
  await page.getByRole('button', { name: 'Resume grant' }).click()
  await page.getByRole('button', { name: 'Retry same grant' }).click()
  await expect(page.getByRole('alert')).toContainText('Reload the dashboard')
  await expect(page.getByText('Unconfirmed grant')).toBeVisible()
  await page.getByRole('button', { name: 'Retry same grant' }).click()
  await expect(page.getByRole('status')).toContainText('BP added')
  expect(ids).toHaveLength(3)
  expect(ids[1]).toBe(ids[0])
  expect(ids[2]).toBe(ids[0])
  const after = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  ).accounts
  expect(after.find((a) => a.user_id === target.user_id)?.bp).toBe(target.bp + 123)
})

test('an expired staff cookie gives a typed error and asks for sign-in rather than sample data', async ({
  page,
}) => {
  await signIn(page)
  await page.context().clearCookies()
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/session'))
  await page.reload()
  expect(ApiError.parse(await (await response).json()).code).toBe('unauthenticated')
  await expect(page.getByText('Sign in to your local server')).toBeVisible()
  await expect(page.getByText('Design preview.')).toHaveCount(0)
})

test('a definitive grant refusal lets the admin correct the amount rather than getting stuck', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('button', { name: 'Give BP to TestPlayer' }).click()
  await page.getByLabel('BP amount').fill('4294967295')
  const reply = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/give-bp'))
  await page.getByRole('button', { name: 'Add BP', exact: true }).click()
  const response = await reply
  expect(response.status()).toBe(400)
  expect(ApiError.parse(await response.json()).code).toBe('invalid_request')
  await expect(page.getByRole('alert')).toContainText('The grant was refused.')
  await expect(page.getByLabel('BP amount')).toBeEnabled()
  await page.getByLabel('BP amount').fill('5')
  await expect(page.getByRole('button', { name: 'Add BP', exact: true })).toBeEnabled()
  await expect(page.getByText('Unconfirmed grant')).toHaveCount(0)
})

test('connected sign-in and accounts remain accessible on a narrow screen', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 850 })
  await page.goto('/connected')
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await signIn(page, 'TestModerator')
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320)
})

test('code creation and removal retain their request IDs after lost responses and reloads', async ({
  page,
}) => {
  await signIn(page)
  await expect(page.getByRole('button', { name: 'Create BP code', exact: true })).toBeVisible({
    timeout: 2000,
  })
  let code = ''
  const createIds: string[] = []
  await page.route('**/panel/api/v1/bp-codes', async (route) => {
    if (route.request().method() !== 'POST') return route.continue()
    createIds.push(route.request().headers()['idempotency-key'])
    if (createIds.length === 2) {
      const refused = await route.fetch({
        headers: { ...route.request().headers(), 'x-csrf-token': 'stale-token' },
      })
      expect(refused.status()).toBe(403)
      expect(ApiError.parse(await refused.json()).code).toBe('csrf_invalid')
      return route.fulfill({ response: refused })
    }
    const response = await route.fetch()
    const result = CreateBpCodeResult.parse(await response.json())
    if (code) expect(result.code).toBe(code)
    code = result.code
    if (createIds.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await page.getByRole('button', { name: 'Create BP code', exact: true }).click()
  await page.getByLabel('BP reward', { exact: true }).fill('250')
  await page.getByRole('button', { name: 'Create code', exact: true }).click()
  await expect(page.getByText('We could not confirm this code change.')).toBeVisible()
  expect(createIds).toHaveLength(1)
  await page.reload()
  await expect(page.getByText('Unconfirmed code change', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Resume code change' }).click()
  await page.getByRole('button', { name: 'Retry same change' }).click()
  await expect(page.getByRole('alert')).toContainText('Reload the dashboard')
  await expect(page.getByText('Unconfirmed code change', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Retry same change' }).click()
  await expect(
    page.getByRole('region', { name: 'BOOSTERPASS codes', exact: true }).getByRole('status'),
  ).toContainText('Code created')
  expect(createIds).toHaveLength(3)
  expect(createIds[1]).toBe(createIds[0])
  expect(createIds[2]).toBe(createIds[0])
  const saved = BpCodeList.parse(await (await page.request.get('/panel/api/v1/bp-codes')).json())
  expect(saved.codes.filter((c) => c.code === code)).toHaveLength(1)

  const removeIds: string[] = []
  await page.route('**/panel/api/v1/bp-codes/remove', async (route) => {
    removeIds.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    expect(RemoveBpCodeResult.parse(await response.json()).removed).toBe(code)
    if (removeIds.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await page.getByRole('button', { name: `Remove ${code}`, exact: true }).click()
  await page.getByRole('button', { name: 'Remove code', exact: true }).click()
  await expect(page.getByText('We could not confirm this code change.')).toBeVisible()
  await page.reload()
  await page.getByRole('button', { name: 'Resume code change' }).click()
  await page.getByRole('button', { name: 'Retry same change' }).click()
  await expect(
    page.getByRole('region', { name: 'BOOSTERPASS codes', exact: true }).getByRole('status'),
  ).toContainText('Code removed')
  expect(removeIds).toHaveLength(2)
  expect(removeIds[1]).toBe(removeIds[0])
  const removed = BpCodeList.parse(await (await page.request.get('/panel/api/v1/bp-codes')).json())
  expect(removed.codes.some((c) => c.code === code)).toBe(false)
})

test('moderators can inspect codes and recover from a failed refresh or expired session', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  const section = page.getByRole('region', { name: 'BOOSTERPASS codes', exact: true })
  await expect(section.getByRole('button', { name: 'Create BP code', exact: true })).toBeDisabled()
  await expect(section.getByRole('button', { name: /^Remove / })).toHaveCount(0)
  expect(
    BpCodeList.safeParse(await (await page.request.get('/panel/api/v1/bp-codes')).json()).success,
  ).toBe(true)
  await page.route('**/panel/api/v1/bp-codes', (route) => route.abort('failed'))
  await section.getByRole('button', { name: 'Refresh codes' }).click()
  await expect(section.getByRole('alert')).toContainText('The code list could not be refreshed.')
  await page.unroute('**/panel/api/v1/bp-codes')
  await page.context().clearCookies()
  await section.getByRole('button', { name: 'Refresh codes' }).click()
  await section.getByRole('button', { name: 'Sign in again' }).click()
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible()
})

test('a duplicate code gives an actionable refusal and leaves the form editable', async ({
  page,
}) => {
  await signIn(page)
  const section = page.getByRole('region', { name: 'BOOSTERPASS codes', exact: true })
  const code = 'UIREWARD20260001'
  await section.getByRole('button', { name: 'Create BP code', exact: true }).click()
  await page.getByLabel('Custom code (optional)', { exact: true }).fill(code)
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await page.getByRole('button', { name: 'Create code', exact: true }).click()
  await expect(section.getByRole('status')).toContainText(`Code created: ${code}`)
  await section.getByRole('button', { name: 'Create BP code', exact: true }).click()
  await page.getByLabel('Custom code (optional)', { exact: true }).fill(code)
  const reply = page.waitForResponse(
    (r) => r.url().endsWith('/panel/api/v1/bp-codes') && r.request().method() === 'POST',
  )
  await page.getByRole('button', { name: 'Create code', exact: true }).click()
  const response = await reply
  expect(response.status()).toBe(409)
  expect(ApiError.parse(await response.json()).code).toBe('already_exists')
  await expect(page.getByRole('alert')).toContainText('The code change was refused.')
  await expect(page.getByLabel('Custom code (optional)', { exact: true })).toBeEnabled()
  await expect(page.getByText('Unconfirmed code change', { exact: true })).toHaveCount(0)
  await page.getByRole('button', { name: 'Close', exact: true }).click()
  await section.getByRole('button', { name: `Remove ${code}`, exact: true }).click()
  await page.getByRole('button', { name: 'Remove code', exact: true }).click()
  await expect(section.getByRole('status')).toContainText(`Code removed: ${code}`)
})

test('invite creation retains its ID and expiry after a lost response, reload and CSRF refusal', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  const section = page.getByRole('region', { name: 'Registration invites', exact: true })
  await section.getByRole('button', { name: 'Create invite', exact: true }).click()
  await page.getByLabel('Sign-up limit', { exact: true }).fill('3')
  await page.getByLabel('Expires after (days)', { exact: true }).fill('7')
  const ids: string[] = []
  let original: unknown
  await page.route('**/panel/api/v1/invites', async (route) => {
    if (route.request().method() !== 'POST') return route.continue()
    ids.push(route.request().headers()['idempotency-key'])
    if (ids.length === 2) {
      const response = await route.fetch({
        headers: { ...route.request().headers(), 'x-csrf-token': 'stale' },
      })
      expect(response.status()).toBe(403)
      return route.fulfill({ response })
    }
    const response = await route.fetch()
    expect(response.status()).toBe(200)
    const body = CreateInviteResult.parse(await response.json())
    if (ids.length === 1) {
      original = body
      return route.abort('failed')
    }
    expect(body).toEqual(original)
    return route.fulfill({ response })
  })
  await page.getByRole('dialog').getByRole('button', { name: 'Create invite', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('We could not confirm this invite')
  expect(ids).toHaveLength(1)
  await page.reload()
  await expect(page.getByText('Unconfirmed invite')).toBeVisible()
  await section.getByRole('button', { name: 'Resume invite' }).click()
  await page.getByRole('button', { name: 'Retry same invite' }).click()
  await expect(page.getByRole('alert')).toContainText('Reload the dashboard')
  await expect(page.getByText('Unconfirmed invite')).toBeVisible()
  await page.getByRole('button', { name: 'Retry same invite' }).click()
  await expect(section.getByRole('status')).toContainText('Invite created')
  expect(ids).toHaveLength(3)
  expect(new Set(ids).size).toBe(1)
})

test('invite limits, empty state, failed refresh and mobile dialog are clear', async ({ page }) => {
  await signIn(page)
  const section = page.getByRole('region', { name: 'Registration invites', exact: true })
  await page.route('**/panel/api/v1/invites', async (route) => {
    if (route.request().method() !== 'GET') return route.continue()
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ invites: [], checked_at: new Date().toISOString() }),
    })
  })
  await section.getByRole('button', { name: 'Refresh invites' }).click()
  await expect(section.getByText('No registration invites yet.')).toBeVisible()
  await section.getByRole('button', { name: 'Create invite', exact: true }).click()
  await page.getByLabel('Sign-up limit', { exact: true }).fill('0')
  await expect(
    page.getByRole('dialog').getByRole('button', { name: 'Create invite', exact: true }),
  ).toBeDisabled()
  await page.getByLabel('Sign-up limit', { exact: true }).fill('')
  await page.getByLabel('Expires after (days)', { exact: true }).fill('')
  await page.setViewportSize({ width: 320, height: 850 })
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await expect(
    page.getByRole('dialog').getByRole('button', { name: 'Create invite', exact: true }),
  ).toBeEnabled()
  await page.getByLabel('Expires after (days)', { exact: true }).fill('4294967295')
  await page.getByRole('dialog').getByRole('button', { name: 'Create invite', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('The invite was refused.')
  await expect(page.getByLabel('Expires after (days)', { exact: true })).toBeEnabled()
  await page.getByLabel('Expires after (days)', { exact: true }).fill('')
  await page.getByRole('dialog').getByRole('button', { name: 'Create invite', exact: true }).click()
  await expect(section.getByRole('status')).toContainText('Invite created')
  InviteList.parse(await (await page.request.get('/panel/api/v1/invites')).json())
  await page.keyboard.press('Escape')
  await page.route('**/panel/api/v1/invites', (route) => route.abort('failed'))
  await section.getByRole('button', { name: 'Refresh invites' }).click()
  await expect(section.getByRole('alert')).toContainText('The invite list could not be refreshed')
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320)
})
