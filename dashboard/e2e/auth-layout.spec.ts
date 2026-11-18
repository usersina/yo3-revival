import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

test('sign-in uses the project logo and a compact, balanced card on every screen size', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' })
  await page.goto('/accounts')
  await expect(page.getByLabel('User ID', { exact: true })).toBeVisible()
  for (const width of [1920, 1440, 768, 390, 320]) {
    await page.setViewportSize({ width, height: 800 })
    const brand = page.getByRole('link', { name: 'YO3 Revival home', exact: true })
    await expect(brand.locator('img')).toBeVisible()
    expect(
      await brand.locator('img').evaluate((img: HTMLImageElement) => img.naturalWidth),
    ).toBeGreaterThan(0)
    const card = page.locator('.auth-card')
    const box = await card.boundingBox()
    expect(box).not.toBeNull()
    expect(box!.width).toBeLessThanOrEqual(480)
    expect(Math.abs(box!.x + box!.width / 2 - width / 2)).toBeLessThan(2)
    const input = await page.getByLabel('User ID', { exact: true }).boundingBox()
    expect(box!.width - input!.width).toBeLessThanOrEqual(66)
    expect(await page.locator('header').evaluate((el) => el.clientHeight)).toBeLessThanOrEqual(80)
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      width,
    )
    await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeInViewport()
  }
  await page.getByLabel('User ID', { exact: true }).fill('   ')
  await page.getByLabel('Password', { exact: true }).fill('example')
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeDisabled()
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})

test('connection, server errors and sign-in errors stay inside the branded entry screen', async ({
  page,
}) => {
  let finishSession: (() => void) | undefined
  const waiting = new Promise<void>((resolve) => {
    finishSession = resolve
  })
  await page.route('**/panel/api/v1/session', async (route) => {
    await waiting
    await route.fulfill({
      status: 503,
      json: { code: 'internal_error', message: 'The server is restarting. Try again shortly.' },
    })
  })
  await page.setViewportSize({ width: 390, height: 650 })
  await page.goto('/')
  await expect(page.getByRole('status')).toContainText('Connecting to your server')
  await expect(
    page.getByRole('link', { name: 'YO3 Revival home', exact: true }).locator('img'),
  ).toBeVisible()
  finishSession!()
  await expect(page.getByRole('heading', { name: 'Server unavailable' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Try again', exact: true })).toBeInViewport()
  await page.unroute('**/panel/api/v1/session')
  await page.getByRole('button', { name: 'Try again', exact: true }).click()
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  // Exercise form recovery without throttling other tests sharing the demo's staff account.
  await page.route('**/panel/api/v1/sign-in', (route) =>
    route.fulfill({
      status: 401,
      json: {
        code: 'unauthenticated',
        message: "Wrong User ID or password, or this account can't use the admin panel.",
      },
    }),
  )
  await page.getByLabel('Password', { exact: true }).fill('incorrect-password')
  const refused = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await refused).status()).toBe(401)
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Sign in', exact: true })).toBeInViewport()
  await expect(page.getByLabel('User ID', { exact: true })).toHaveValue('TestAdmin')
})

test('an expired session keeps a refused sign-in visible instead of restoring stale pages', async ({
  page,
}) => {
  await page.goto('/accounts')
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  await expect(
    page.getByRole('button', { name: 'View account TestPlayer', exact: true }),
  ).toBeVisible({ timeout: 15000 })
  await page.context().clearCookies()
  await page.getByRole('button', { name: 'Refresh', exact: true }).click()
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  // Exercise form recovery without throttling other tests sharing the demo's staff account.
  await page.route('**/panel/api/v1/sign-in', (route) =>
    route.fulfill({
      status: 401,
      json: {
        code: 'unauthenticated',
        message: "Wrong User ID or password, or this account can't use the admin panel.",
      },
    }),
  )
  await page.getByLabel('Password', { exact: true }).fill('incorrect-password')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await response).status()).toBe(401)
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByLabel('User ID', { exact: true })).toHaveValue('TestAdmin')
  await expect(page).toHaveTitle('Sign in · YO3 Revival')
})
