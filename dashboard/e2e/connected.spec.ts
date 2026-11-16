import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import {
  AccountList,
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
  await expect(page.getByRole('status')).toContainText('BP added')
  expect(ids).toHaveLength(2)
  expect(ids[1]).toBe(ids[0])
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
