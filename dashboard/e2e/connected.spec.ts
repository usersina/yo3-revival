import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import {
  AccountList,
  GiftResult,
  GiftChoices,
  KeyAccountResult,
  AccountCodeResponse,
  AccountDetail,
  AccountEditResult,
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

test('account profiles: add and rename keep stable IDs through lost responses and reloads', async ({
  page,
}) => {
  await signIn(page)
  const detailResponse = page.waitForResponse((r) =>
    /\/panel\/api\/v1\/accounts\/\d+$/.test(r.url()),
  )
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  const detail = AccountDetail.parse(await (await detailResponse).json())
  expect(detail.login).toBe('TestPlayer')
  let dialog = page.getByRole('dialog', { name: 'TestPlayer', exact: true })
  await expect(dialog.getByText('Additional cards', { exact: true })).toBeVisible()
  await dialog.getByRole('button', { name: 'Add player', exact: true }).click()
  await dialog.getByLabel('Player name', { exact: true }).fill('ProfileJoey')
  const ids: string[] = []
  let original: unknown
  await page.route('**/panel/api/v1/accounts/edit', async (route) => {
    const request = route.request().postDataJSON()
    if (request.kind !== 'add_player') return route.continue()
    ids.push(route.request().headers()['idempotency-key'])
    if (ids.length === 2) {
      const refused = await route.fetch({
        headers: { ...route.request().headers(), 'x-csrf-token': 'stale' },
      })
      expect(refused.status()).toBe(403)
      return route.fulfill({ response: refused })
    }
    const response = await route.fetch()
    expect(response.status()).toBe(200)
    const body = AccountEditResult.parse(await response.json())
    if (ids.length === 1) {
      original = body
      return route.abort('failed')
    }
    expect(body).toEqual(original)
    return route.fulfill({ response })
  })
  await dialog.getByRole('button', { name: 'Add player', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('We could not confirm this profile change')
  await page.reload()
  await expect(page.getByText('Unconfirmed profile change')).toBeVisible()
  await page.getByRole('button', { name: 'Resume profile change' }).click()
  dialog = page.getByRole('dialog', { name: 'TestPlayer', exact: true })
  await dialog.getByRole('button', { name: 'Retry same change', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('Reload the dashboard')
  await dialog.getByRole('button', { name: 'Retry same change', exact: true }).click()
  await expect(dialog.getByRole('status')).toContainText('Profile saved')
  expect(ids).toHaveLength(3)
  expect(ids[2]).toBe(ids[0])
  expect(ids[0]).toBe(ids[1])
  await expect(dialog.getByText('ProfileJoey', { exact: true })).toHaveCount(1)
  await dialog.getByRole('button', { name: 'Rename ProfileJoey', exact: true }).click()
  await dialog.getByLabel('Player name', { exact: true }).fill('ProfileAtem')
  await dialog.getByRole('button', { name: 'Save name', exact: true }).click()
  await expect(dialog.getByText('ProfileAtem', { exact: true })).toBeVisible()
  await dialog.getByRole('button', { name: 'Change country', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('Country choices are unavailable')
  await dialog.getByLabel('Country', { exact: true }).selectOption('')
  await dialog.getByRole('button', { name: 'Save country', exact: true }).click()
  await expect(dialog.getByRole('status')).toContainText('Profile saved')
})

test('account profiles: moderators see explicit permissions and mobile details handle failures', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await page.getByRole('button', { name: 'View account TestAdmin', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'TestAdmin', exact: true })
  await expect(dialog.getByRole('button', { name: 'Add player', exact: true })).toBeDisabled()
  await expect(dialog.getByRole('button', { name: 'Change country', exact: true })).toBeDisabled()
  await expect(dialog.getByRole('button', { name: 'Rename TestAdmin', exact: true })).toBeDisabled()
  await page.setViewportSize({ width: 320, height: 850 })
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320)
  await page.route('**/panel/api/v1/accounts/*', (route) => route.abort('failed'))
  await dialog.getByRole('button', { name: 'Refresh account', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('Account details could not be refreshed')
  await page.keyboard.press('Escape')
  await page.unroute('**/panel/api/v1/accounts/*')
  await page.route('**/panel/api/v1/accounts/*', (route) =>
    route.fulfill({
      status: 404,
      contentType: 'application/json',
      body: JSON.stringify({ code: 'not_found', message: 'The account no longer exists.' }),
    }),
  )
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText(
    'The account no longer exists',
  )
})

test('account profiles: a saved country edit stays explicit when country choices are unavailable', async ({
  page,
}) => {
  await signIn(page)
  const session = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  const account = AccountList.parse(
    await (await page.request.get('/panel/api/v1/accounts')).json(),
  ).accounts.find((a) => a.login === 'TestPlayer')!
  await page.evaluate(
    ({ staff, user }) =>
      sessionStorage.setItem(
        `yo3:pending-profile:${staff}`,
        JSON.stringify({
          data: { kind: 'set_country', user_id: user, country: 'JP' },
          headers: { 'Idempotency-Key': 'saved-country-browser-00001' },
        }),
      ),
    { staff: session.user_id, user: account.user_id },
  )
  await page.reload()
  await page.getByRole('button', { name: 'Resume profile change' }).click()
  const dialog = page.getByRole('dialog', { name: 'TestPlayer', exact: true })
  await expect(dialog.getByLabel('Country', { exact: true })).toHaveValue('JP')
  await expect(dialog.getByLabel('Country', { exact: true })).toBeDisabled()
  await page.setViewportSize({ width: 320, height: 850 })
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await dialog.getByRole('button', { name: 'Retry same change', exact: true }).click()
  await expect(dialog.getByText('We could not confirm this profile change.')).toBeVisible()
  await expect(dialog.getByLabel('Country', { exact: true })).toHaveValue('JP')
})

test('role changes require confirmation and retain their ID after a lost response and reload', async ({
  page,
}) => {
  await signIn(page)
  const ids: string[] = []
  await page.route('**/panel/api/v1/accounts/edit', async (route) => {
    const data = route.request().postDataJSON() as { kind: string }
    if (data.kind !== 'set_role') return route.continue()
    ids.push(route.request().headers()['idempotency-key'])
    if (ids.length === 1) {
      const response = await route.fetch()
      expect(AccountEditResult.safeParse(await response.json()).success).toBe(true)
      return route.abort('failed')
    }
    return route.continue()
  })
  await page.getByRole('button', { name: 'View account TestRoleTarget', exact: true }).click()
  let dialog = page.getByRole('dialog', { name: 'TestRoleTarget', exact: true })
  await dialog.getByRole('button', { name: 'Change role', exact: true }).click()
  await dialog.getByLabel('New role', { exact: true }).selectOption('moderator')
  await expect(dialog.getByRole('button', { name: 'Save role', exact: true })).toBeDisabled()
  await dialog.getByRole('checkbox', { name: /I confirm this role change/ }).check()
  await dialog.getByRole('button', { name: 'Save role', exact: true }).click()
  await expect(dialog.getByText('We could not confirm this profile change.')).toBeVisible()
  await page.reload()
  await page.getByRole('button', { name: 'Resume profile change', exact: true }).click()
  dialog = page.getByRole('dialog', { name: 'TestRoleTarget', exact: true })
  await expect(dialog.getByLabel('New role', { exact: true })).toHaveValue('moderator')
  await expect(dialog.getByLabel('New role', { exact: true })).toBeDisabled()
  for (let attempt = 0; attempt < 2; attempt++) {
    const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/accounts/edit'))
    await dialog.getByRole('button', { name: 'Retry same change', exact: true }).click()
    const status = (await response).status()
    if (status === 200) break
    expect(status).toBe(403)
    await page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/session'))
  }
  await expect(dialog.getByText('Role saved. Changed accounts must sign in again.')).toBeVisible()
  expect(ids.length).toBeGreaterThanOrEqual(2)
  expect(new Set(ids).size).toBe(1)
  await page.setViewportSize({ width: 320, height: 850 })
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})

test('moderators cannot change roles and changing your own role signs you out', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  await expect(
    page.getByRole('dialog').getByRole('button', { name: 'Change role', exact: true }),
  ).toBeDisabled()
  await page.getByRole('dialog').getByRole('button', { name: 'Close', exact: true }).click()
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await signIn(page, 'TestRoleAdmin')
  await page.getByRole('button', { name: 'View account TestRoleAdmin', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'TestRoleAdmin', exact: true })
  await dialog.getByRole('button', { name: 'Change role', exact: true }).click()
  await dialog.getByLabel('New role', { exact: true }).selectOption('moderator')
  await expect(
    dialog.getByText('Changing your own role signs you out. You will lose administrator access.'),
  ).toBeVisible()
  await dialog.getByRole('checkbox', { name: /I confirm this role change/ }).check()
  await dialog.getByRole('button', { name: 'Save role', exact: true }).click()
  await expect(page.getByLabel('Password', { exact: true })).toBeVisible()
  await signIn(page, 'TestRoleAdmin')
  await page.getByRole('button', { name: 'View account TestRoleAdmin', exact: true }).click()
  await expect(
    page.getByRole('dialog').getByRole('button', { name: 'Change role', exact: true }),
  ).toBeDisabled()
})

test('account setup: lost response and reload retain the same code and request ID', async ({
  page,
}) => {
  await signIn(page)
  const sent: { code: string; id: string }[] = []
  await page.route('**/panel/api/v1/accounts/credentials', async (route) => {
    const data = route.request().postDataJSON() as { code: string }
    sent.push({ code: data.code, id: route.request().headers()['idempotency-key'] })
    if (sent.length === 1) {
      const result = AccountCodeResponse.parse(await (await route.fetch()).json())
      expect(result.state).toBe('ready')
      expect(JSON.stringify(result)).not.toContain(data.code)
      return route.abort('failed')
    }
    return route.continue()
  })
  await page.getByRole('button', { name: 'Create account', exact: true }).click()
  await page.getByRole('dialog').getByLabel('User ID', { exact: true }).fill('NewOwner')
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Create account', exact: true })
    .click()
  await expect(
    page.getByRole('dialog').getByText('Could not confirm this code request.'),
  ).toBeVisible()
  await expect(page.getByLabel('One-time code', { exact: true })).toHaveCount(0)
  await page.reload()
  await page.getByRole('button', { name: 'Resume code', exact: true }).click()
  for (let attempt = 0; attempt < 2; attempt++) {
    const response = page.waitForResponse((r) =>
      r.url().endsWith('/panel/api/v1/accounts/credentials'),
    )
    await page
      .getByRole('dialog')
      .getByRole('button', { name: 'Retry same request', exact: true })
      .click()
    const status = (await response).status()
    if (status === 200) break
    expect(status).toBe(403)
    await page.waitForLoadState('networkidle')
  }
  await expect(page.getByLabel('One-time code', { exact: true })).toHaveText(sent[0].code)
  expect(new Set(sent.map((r) => r.id)).size).toBe(1)
  expect(new Set(sent.map((r) => r.code)).size).toBe(1)
  await page.setViewportSize({ width: 320, height: 850 })
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await page.getByRole('dialog').getByRole('button', { name: 'Done', exact: true }).click()
  expect(
    await page.evaluate(() =>
      Object.keys(sessionStorage).some((key) => key.startsWith('yo3:account-code:')),
    ),
  ).toBe(false)
  await expect(
    page.getByRole('button', { name: 'View account newowner', exact: true }),
  ).toBeVisible()
})

test('reset codes require identity confirmation and cannot be shared after replacement', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await expect(page.getByRole('button', { name: 'Create account', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: 'View account TestResetTarget', exact: true }).click()
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Reset password', exact: true })
    .click()
  let dialog = page.getByRole('dialog', { name: 'Reset password', exact: true })
  await expect(
    dialog.getByRole('button', { name: 'Create reset code', exact: true }),
  ).toBeDisabled()
  await dialog.getByRole('checkbox', { name: /I have verified/ }).check()
  const confirmed = page.waitForResponse((r) =>
    r.url().endsWith('/panel/api/v1/accounts/credentials'),
  )
  await dialog.getByRole('button', { name: 'Create reset code', exact: true }).click()
  const original = AccountCodeResponse.parse(await (await confirmed).json())
  await expect(page.getByLabel('One-time code', { exact: true })).toBeVisible()
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  await page.reload()
  await page.getByRole('button', { name: 'Resume code', exact: true }).click()
  dialog = page.getByRole('dialog', { name: 'Reset password', exact: true })
  await expect(page.getByLabel('One-time code', { exact: true })).toHaveCount(0)
  const staff = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  const replacement = await page.request.post('/panel/api/v1/accounts/credentials', {
    headers: {
      'X-CSRF-Token': staff.csrf_token,
      Origin: new URL(page.url()).origin,
      'Idempotency-Key': 'browser-replace-code-00001',
    },
    data: {
      kind: 'reset_password',
      user_id: original.confirmation.user_id,
      code: 'QRST-UVWX-YZ23-4567',
    },
  })
  expect(replacement.status()).toBe(200)
  for (let attempt = 0; attempt < 2; attempt++) {
    const response = page.waitForResponse((r) =>
      r.url().endsWith('/panel/api/v1/accounts/credentials'),
    )
    await dialog.getByRole('button', { name: 'Retry same request', exact: true }).click()
    const status = (await response).status()
    if (status === 200) break
    expect(status).toBe(403)
    await page.waitForLoadState('networkidle')
  }
  await expect(dialog.getByText(/This code was used, replaced or invalidated/)).toBeVisible()
  await expect(page.getByLabel('One-time code', { exact: true })).toHaveCount(0)
  await dialog.getByRole('button', { name: 'Done', exact: true }).click()
  await page.getByRole('button', { name: 'View account TestAdmin', exact: true }).click()
  await expect(
    page.getByRole('dialog').getByRole('button', { name: 'Reset password', exact: true }),
  ).toBeDisabled()
})

test('account setup errors keep the form useful and expired confirmations discard the stored code', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('button', { name: 'Create account', exact: true }).click()
  let dialog = page.getByRole('dialog')
  await dialog.getByLabel('User ID', { exact: true }).fill('ab')
  await expect(dialog.getByRole('button', { name: 'Create account', exact: true })).toBeDisabled()
  await dialog.getByLabel('User ID', { exact: true }).fill('TestPlayer')
  await dialog.getByRole('button', { name: 'Create account', exact: true }).click()
  await expect(dialog.getByText('That User ID is already taken.')).toBeVisible()
  await dialog.getByRole('button', { name: 'Edit request', exact: true }).click()
  await expect(dialog.getByLabel('User ID', { exact: true })).toHaveValue('TestPlayer')
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  const staff = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  await page.evaluate(
    (staffId) =>
      sessionStorage.setItem(
        `yo3:account-code:${staffId}`,
        JSON.stringify({
          label: 'ExpiredUser',
          intent: {
            data: { kind: 'create_account', user: 'ExpiredUser', code: 'ABCD-EFGH-JKLM-NPQR' },
            headers: { 'Idempotency-Key': 'expired-browser-code-00001' },
          },
          result: {
            confirmation: {
              kind: 'setup',
              user_id: 1234,
              expires_at: '2000-01-01T00:00:00Z',
              request_id: 'expired-browser-code-00001',
            },
            state: 'ready',
          },
        }),
      ),
    staff.user_id,
  )
  await page.reload()
  await page.getByRole('button', { name: 'Resume code', exact: true }).click()
  dialog = page.getByRole('dialog')
  await expect(dialog.getByText(/This code has expired/)).toBeVisible()
  await expect
    .poll(() =>
      page.evaluate(
        (staffId) =>
          sessionStorage.getItem(`yo3:account-code:${staffId}`)?.includes('ABCD-EFGH-JKLM-NPQR') ??
          false,
        staff.user_id,
      ),
    )
    .toBe(false)
  await expect(page.getByLabel('One-time code', { exact: true })).toHaveCount(0)
})

test('key accounts survive a lost response and reload; keys are explicit and never retained in browser storage', async ({
  page,
}) => {
  await signIn(page)
  const ids: string[] = []
  let secret = ''
  await page.route('**/panel/api/v1/accounts/keys', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    expect(response.status()).toBe(200)
    const data = KeyAccountResult.parse(await response.json())
    secret = data.key
    if (ids.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await page.getByRole('button', { name: 'Create key account', exact: true }).click()
  await page.getByLabel('Player names').fill('BrowserKey')
  await page.getByLabel('I understand this key allows passwordless game access').check()
  await page.getByRole('button', { name: 'Create and reveal key' }).click()
  await expect(page.getByRole('alert')).toContainText('Could not reach')
  await page.reload()
  await page.getByRole('button', { name: 'Resume key request' }).click()
  await page.getByRole('button', { name: 'Retry same request' }).click()
  await expect(page.getByLabel('Account key', { exact: true })).toHaveValue(secret)
  expect(ids[0]).toBe(ids[1])
  const storage = await page.evaluate(() => JSON.stringify({ ...sessionStorage, ...localStorage }))
  expect(storage).not.toContain(secret)
  await expect(page.getByRole('dialog')).toContainText('BrowserKey')
  await page.getByRole('button', { name: 'Done', exact: true }).click()
  await page
    .getByRole('row')
    .filter({ hasText: 'BrowserKey' })
    .getByRole('button', { name: /View account/ })
    .click()
  await page.getByRole('button', { name: 'Reveal key', exact: true }).click()
  await page.getByLabel('I understand this key allows passwordless game access').check()
  await page.getByRole('button', { name: 'Reveal account key' }).click()
  await expect(page.getByLabel('Account key', { exact: true })).toHaveValue(secret)
  await page.getByRole('button', { name: 'Done', exact: true }).click()
  await expect(page.getByLabel('Account key', { exact: true })).toHaveCount(0)
})

test('key controls respect moderator permissions and the creation form fits narrow screens', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await expect(page.getByRole('button', { name: 'Create key account', exact: true })).toBeDisabled()
  await page.getByRole('button', { name: 'View account TestPlayer' }).click()
  await expect(page.getByRole('button', { name: 'Reveal key', exact: true })).toBeDisabled()
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await signIn(page)
  await page.setViewportSize({ width: 390, height: 850 })
  await page.getByRole('button', { name: 'Create key account', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Create and reveal key' })).toBeDisabled()
  await page.getByLabel('Player names').fill('One, Two, Three, Four, Five')
  await page.getByLabel('I understand this key allows passwordless game access').check()
  await page.getByRole('button', { name: 'Create and reveal key' }).click()
  await expect(page.getByRole('alert')).toContainText('at most four')
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([])
  const dialog = await page.getByRole('dialog').boundingBox()
  expect(dialog!.x).toBeGreaterThanOrEqual(0)
  expect(dialog!.x + dialog!.width).toBeLessThanOrEqual(390)
  await page.screenshot({ path: '/tmp/yo3-key-account-mobile.png', animations: 'disabled' })
})

test('player gifts retain one request across a lost response and reload, including card mail and recipe/title choices', async ({
  page,
}) => {
  await signIn(page)
  const ids: string[] = []
  await page.route('**/panel/api/v1/gifts', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    expect(response.status()).toBe(200)
    expect(GiftResult.safeParse(await response.json()).success).toBe(true)
    if (ids.length === 1) await route.abort('failed')
    else await route.fulfill({ response })
  })
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
  const catalogue = await page.request.get('/panel/api/v1/gifts/choices?q=Blue-Eyes')
  expect(GiftChoices.parse(await catalogue.json()).cards.length).toBe(2)
  await page.getByLabel('Gift', { exact: true }).selectOption('816')
  await page.getByLabel('Copies', { exact: true }).fill('3')
  await page.getByLabel('Confirm this gift for the selected player').check()
  await page.getByRole('button', { name: 'Give gift', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Could not reach')
  await page.reload()
  await page.getByRole('button', { name: 'Resume gift', exact: true }).click()
  await page.getByRole('button', { name: 'Retry same request', exact: true }).click()
  await expect(page.getByRole('dialog')).toContainText('Additional copies of artwork #816: 3')
  expect(ids[0]).toBe(ids[1])
  await page.getByRole('button', { name: 'Done', exact: true }).click()
  for (const kind of ['recipe', 'title']) {
    await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
    await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
    await page.getByLabel('Gift type').selectOption(kind)
    await page.getByLabel('Gift', { exact: true }).selectOption(kind === 'recipe' ? '7' : '300')
    await page.getByLabel('Confirm this gift for the selected player').check()
    await page.getByRole('button', { name: 'Give gift', exact: true }).click()
    await expect(page.getByRole('dialog')).toContainText(
      kind === 'recipe' ? 'Recipe #7 saved without its cards' : 'Title #300 saved',
    )
    await page.getByRole('button', { name: 'Done', exact: true }).click()
  }
})

test('gift permissions, search, catalogue failures and mobile confirmation remain actionable', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Gift to TestPlayer', exact: true })).toBeDisabled()
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await signIn(page)
  await page.setViewportSize({ width: 390, height: 850 })
  await page.getByRole('button', { name: 'View account TestResetTarget', exact: true }).click()
  await page.getByRole('button', { name: 'Gift to TestResetTarget', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Give gift', exact: true })).toBeDisabled()
  await page.route('**/panel/api/v1/gifts/choices*', (route) =>
    route.fulfill({
      status: 503,
      contentType: 'application/json',
      body: JSON.stringify({
        code: 'persistence_failed',
        message: 'Gift choices are unavailable.',
      }),
    }),
  )
  await page.getByLabel('Find a gift', { exact: true }).fill('missing')
  await expect(page.getByRole('alert')).toContainText('Gift choices are unavailable')
  await expect(page.getByRole('button', { name: 'Give gift', exact: true })).toBeDisabled()
  await page.unroute('**/panel/api/v1/gifts/choices*')
  await page.getByRole('button', { name: 'Retry choices', exact: true }).click()
  await expect(page.getByText('No matching gifts. Try another name or ID.')).toBeVisible()
  await page.getByLabel('Find a gift', { exact: true }).fill('4007')
  await page.getByLabel('Gift', { exact: true }).selectOption('817')
  await page.getByLabel('Copies', { exact: true }).fill('2')
  await page.getByLabel('Confirm this gift for the selected player').check()
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([])
  const box = await page.getByRole('dialog').boundingBox()
  expect(box!.x).toBeGreaterThanOrEqual(0)
  expect(box!.x + box!.width).toBeLessThanOrEqual(390)
  await page.screenshot({ path: '/tmp/yo3-gift-mobile.png', animations: 'disabled' })
  await page.getByRole('button', { name: 'Give gift', exact: true }).click()
  await expect(page.getByRole('dialog')).toContainText('Additional copies of artwork #817: 2')
})
