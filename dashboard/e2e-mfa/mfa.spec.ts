import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { createHmac } from 'node:crypto'
import { SignInOutcome, StaffSession } from '../src/api/generated/validation/schemas'

const PASSWORD = 'DashboardDemo123!'

function base32decode(text: string) {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567'
  let bits = 0
  let value = 0
  const out: number[] = []
  for (const c of text.replace(/[\s=]/g, '').toUpperCase()) {
    value = (value << 5) | alphabet.indexOf(c)
    bits += 5
    if (bits >= 8) {
      bits -= 8
      out.push((value >>> bits) & 0xff)
    }
  }
  return Buffer.from(out)
}

/** RFC 6238 (SHA-1, 6 digits, 30 s) as an authenticator app computes it; `ahead` steps into the future. */
function totp(secret: string, ahead = 0) {
  const step = Math.floor(Date.now() / 30000) + ahead
  const counter = Buffer.alloc(8)
  counter.writeBigUInt64BE(BigInt(step))
  const hmac = createHmac('sha1', base32decode(secret)).update(counter).digest()
  const offset = hmac[19] & 15
  return String((hmac.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0')
}

async function password(page: Page, user: string) {
  await page.goto('/')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill(PASSWORD)
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  return SignInOutcome.parse(await (await response).json())
}

/** Signs in with a password only (an account with no authenticator) and enrolls: its secret and recovery codes. */
async function enroll(page: Page, user: string) {
  const outcome = await password(page, user)
  expect(outcome.code_required).toBe(false)
  expect(outcome.session?.limited).toBe(true)
  await expect(page.getByRole('heading', { name: 'Set up two-step sign-in' })).toBeVisible()
  await page.getByRole('button', { name: 'Set up authenticator' }).click()
  await expect(page.getByRole('img', { name: 'QR code for your authenticator app' })).toBeVisible()
  const secret = (await page.getByTestId('mfa-secret').innerText()).replace(/\s/g, '')
  await page.getByLabel('Code from your app').fill(totp(secret))
  await page.getByRole('button', { name: 'Activate' }).click()
  await expect(page.getByRole('list', { name: 'Recovery codes' })).toBeVisible()
  const codes = await page
    .getByRole('list', { name: 'Recovery codes' })
    .locator('code')
    .allInnerTexts()
  expect(codes).toHaveLength(10)
  await page.getByLabel('I saved these recovery codes').check()
  await page.getByRole('button', { name: 'Continue to the dashboard' }).click()
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible()
  return { secret, codes }
}

async function signOut(page: Page) {
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible()
}

test('an account with no authenticator is held to enrollment, then signs in with password and code', async ({
  page,
}) => {
  const { secret } = await enroll(page, 'MfaAdmin1')
  const session = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  expect(session).toMatchObject({ mfa_enabled: true, limited: false, recovery_codes_remaining: 10 })
  await signOut(page)
  const outcome = await password(page, 'MfaAdmin1')
  expect(outcome.code_required).toBe(true)
  expect(outcome.session).toBeNull()
  await expect(page.getByRole('heading', { name: 'Enter your code' })).toBeVisible()
  // A wrong code is a form error; the right one (the next 30-second step: enrollment used this one) signs in.
  await page.getByLabel('Authenticator code').fill('000000')
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('alert')).toContainText('not right')
  await page.waitForTimeout(1200)
  await page.getByLabel('Authenticator code').fill(totp(secret, 1))
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible()
})

test('the enrollment-only session reaches nothing else, wrong codes activate nothing', async ({
  page,
}) => {
  await password(page, 'MfaAdmin2')
  await expect(page.getByRole('heading', { name: 'Set up two-step sign-in' })).toBeVisible()
  await expect(page.getByRole('navigation', { name: 'Main navigation' })).toHaveCount(0)
  for (const path of ['accounts', 'server', 'backups']) {
    const response = await page.request.get(`/panel/api/v1/${path}`)
    expect(response.status()).toBe(403)
    expect((await response.json()).code).toBe('mfa_enrollment_required')
  }
  await page.getByRole('button', { name: 'Set up authenticator' }).click()
  await page.getByLabel('Code from your app').fill('000000')
  await page.getByRole('button', { name: 'Activate' }).click()
  await expect(page.getByRole('alert')).toContainText('not right')
  await expect(page.getByRole('heading', { name: 'Set up two-step sign-in' })).toBeVisible()
  // Reloading keeps the session limited, not signed out and not promoted.
  await page.reload()
  await expect(page.getByRole('heading', { name: 'Set up two-step sign-in' })).toBeVisible()
})

test('a recovery code signs in once and is then spent', async ({ page }) => {
  const { codes } = await enroll(page, 'MfaAdmin3')
  await signOut(page)
  await password(page, 'MfaAdmin3')
  await page.getByRole('button', { name: 'Use a recovery code' }).click()
  await page.getByLabel('Recovery code').fill(codes[0].toLowerCase())
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible()
  const session = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  expect(session.recovery_codes_remaining).toBe(9)
  await signOut(page)
  await password(page, 'MfaAdmin3')
  await page.getByRole('button', { name: 'Use a recovery code' }).click()
  await page.getByLabel('Recovery code').fill(codes[0])
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('alert')).toContainText('not right')
  await page.getByRole('button', { name: 'Back to password' }).click()
  await expect(page.getByLabel('Password', { exact: true })).toBeVisible()
})

test('a sensitive action that needs a recent check opens the step-up dialog, which accepts a code', async ({
  page,
}) => {
  const { codes } = await enroll(page, 'MfaAdmin4')
  let refused = 0
  await page.route('**/panel/api/v1/backups', async (route) => {
    if (route.request().method() !== 'POST' || refused > 0) return route.continue()
    refused += 1
    await route.fulfill({
      status: 403,
      json: {
        code: 'reauthentication_required',
        message: 'Confirm with your authenticator code to do this.',
      },
    })
  })
  await page.goto('/backups')
  await page.getByRole('button', { name: 'Create backup', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Confirm it is you' })
  await expect(dialog).toBeVisible()
  await dialog.getByLabel('Authenticator or recovery code').fill('not-a-code')
  await dialog.getByRole('button', { name: 'Confirm' }).click()
  await expect(dialog.getByRole('alert')).toContainText('not right')
  await page.waitForTimeout(1200)
  await dialog.getByLabel('Authenticator or recovery code').fill(codes[1])
  await dialog.getByRole('button', { name: 'Confirm' }).click()
  await expect(dialog.getByRole('status')).toContainText('Confirmed')
  await dialog.getByRole('button', { name: 'Close', exact: true }).first().click()
  // The page kept the refused request; checking it sends the same one again, now allowed.
  await page.getByRole('button', { name: 'Check backup outcome', exact: true }).click()
  await expect(
    page.getByRole('region', { name: 'Database backups' }).getByRole('status'),
  ).toContainText('Backup created.')
})

test('settings show the second factor, make new recovery codes and sign out everywhere', async ({
  page,
}) => {
  const { codes } = await enroll(page, 'MfaAdmin5')
  await page.goto('/settings')
  const panel = page.locator('section.security-panel')
  await expect(panel).toContainText('On')
  await expect(panel).toContainText('10 left')
  await panel.getByRole('button', { name: 'New recovery codes' }).click()
  const dialog = page.getByRole('dialog', { name: 'New recovery codes' })
  const fresh = await dialog
    .getByRole('list', { name: 'Recovery codes' })
    .locator('code')
    .allInnerTexts()
  expect(fresh).toHaveLength(10)
  expect(fresh).not.toContain(codes[0])
  await dialog.getByLabel('I saved these recovery codes').check()
  await dialog.getByRole('button', { name: 'Done' }).click()
  await panel.getByRole('button', { name: 'Sign out everywhere' }).click()
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeVisible()
  // The old recovery codes no longer work.
  await password(page, 'MfaAdmin5')
  await page.getByRole('button', { name: 'Use a recovery code' }).click()
  await page.getByLabel('Recovery code').fill(codes[2])
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('alert')).toContainText('not right')
  await page.waitForTimeout(2200)
  await page.getByLabel('Recovery code').fill(fresh[0])
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  await expect(page.getByRole('heading', { name: 'Overview', exact: true })).toBeVisible()
})

for (const scheme of ['light', 'dark'] as const) {
  test(`the enrollment, code and recovery screens fit phones and pass accessibility checks in ${scheme} mode`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: 'reduce' })
    const user = scheme === 'light' ? 'MfaAdmin6' : 'MfaAdmin7'
    await password(page, user)
    await page.getByRole('button', { name: 'Set up authenticator' }).click()
    await expect(page.getByTestId('mfa-secret')).toBeVisible()
    const secret = (await page.getByTestId('mfa-secret').innerText()).replace(/\s/g, '')
    for (const width of [1440, 768, 390, 320]) {
      await page.setViewportSize({ width, height: 900 })
      const card = await page.locator('.auth-card').boundingBox()
      expect(card!.width).toBeLessThanOrEqual(width)
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
        width,
      )
      const qr = await page
        .getByRole('img', { name: 'QR code for your authenticator app' })
        .boundingBox()
      expect(qr!.width).toBeGreaterThan(100)
      expect(qr!.x + qr!.width).toBeLessThanOrEqual(width)
    }
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
    await page.getByLabel('Code from your app').fill(totp(secret))
    await page.getByRole('button', { name: 'Activate' }).click()
    await expect(page.getByRole('list', { name: 'Recovery codes' })).toBeVisible()
    for (const width of [390, 320]) {
      await page.setViewportSize({ width, height: 900 })
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
        width,
      )
    }
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
    await page.getByLabel('I saved these recovery codes').check()
    await page.getByRole('button', { name: 'Continue to the dashboard' }).click()
    await signOut(page)
    await password(page, user)
    await expect(page.getByRole('heading', { name: 'Enter your code' })).toBeVisible()
    for (const width of [390, 320]) {
      await page.setViewportSize({ width, height: 650 })
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
        width,
      )
      await expect(page.getByRole('button', { name: 'Verify and sign in' })).toBeInViewport()
    }
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
  })
}

test('the session cookie is HttpOnly and scoped, and the code step needs the same origin and an idempotency key', async ({
  page,
}) => {
  await enroll(page, 'MfaAdmin8')
  await signOut(page)
  const outcome = await password(page, 'MfaAdmin8')
  expect(outcome.challenge).toHaveLength(64)
  const origin = new URL(page.url()).origin
  const base = {
    'Content-Type': 'application/json',
    'Idempotency-Key': 'e2e-mfa-origin-check-0001',
  }
  const body = JSON.stringify({ challenge: outcome.challenge, code: '000000' })
  const foreign = await page.request.post('/panel/api/v1/sign-in/code', {
    headers: { ...base, Origin: 'https://evil.example' },
    data: body,
  })
  expect(foreign.status()).toBe(403)
  expect((await foreign.json()).code).toBe('invalid_origin')
  const keyless = await page.request.post('/panel/api/v1/sign-in/code', {
    headers: { 'Content-Type': 'application/json', Origin: origin },
    data: body,
  })
  expect(keyless.status()).toBe(400)
  await page.getByLabel('Authenticator code').fill('000000')
  const wrong = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in/code'))
  await page.getByRole('button', { name: 'Verify and sign in' }).click()
  expect((await wrong).status()).toBe(403)
  const cookies = await page.context().cookies()
  expect(cookies.find((c) => c.name === 'yo3_panel')).toBeUndefined()
  expect(await page.evaluate(() => document.cookie)).toBe('')
})
