import { chooseOption } from './select'
import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

async function signIn(page: Page) {
  await page.goto('/settings')
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const signedIn = page.waitForResponse(
    (response) =>
      response.url().endsWith('/panel/api/v1/sign-in') && response.request().method() === 'POST',
  )
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await signedIn).status()).toBe(200)
  await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible()
}

test('theme follows the system until chosen, persists and stays in sync across tabs', async ({
  page,
  context,
}) => {
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/settings')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await page.getByRole('button', { name: 'Switch to light mode', exact: true }).click()
  await page.reload()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await signIn(page)
  await expect(page.getByLabel('Color theme', { exact: true })).toHaveAttribute(
    'data-value',
    'light',
  )
  const other = await context.newPage()
  await other.goto('/accounts')
  await expect(other.locator('html')).toHaveAttribute('data-theme', 'light')
  await chooseOption(page.getByLabel('Color theme', { exact: true }), 'dark')
  await expect(other.locator('html')).toHaveAttribute('data-theme', 'dark')
  await chooseOption(page.getByLabel('Color theme', { exact: true }), 'system')
  await page.emulateMedia({ colorScheme: 'light' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await other.close()
})

test('theme switching still works when browser storage is blocked', async ({ page }) => {
  await page.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw new Error('Storage blocked')
    }
    Storage.prototype.setItem = () => {
      throw new Error('Storage blocked')
    }
  })
  await page.emulateMedia({ colorScheme: 'light' })
  await page.goto('/')
  await page.getByRole('button', { name: 'Switch to dark mode', exact: true }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await expect(page.getByLabel('User ID', { exact: true })).toBeVisible()
})

test('settings group real metadata, retain unknown settings and offer useful filters', async ({
  page,
}) => {
  await page.route('**/panel/api/v1/settings', async (route) => {
    if (route.request().method() !== 'GET') return route.continue()
    const response = await route.fetch()
    const data = await response.json()
    data.settings.push({
      ...data.settings[0],
      name: 'future-setting',
      help: 'A new server setting',
      value: 'Future value',
      overridden: true,
    })
    await route.fulfill({ response, json: data })
  })
  await signIn(page)
  for (const name of [
    'Appearance',
    'Server & access',
    'Duels',
    'Capacity & traffic',
    'Sign-in protection',
    'Maintenance',
    'Audit & other',
  ])
    await expect(page.getByRole('heading', { name: new RegExp(`^${name}`) })).toBeVisible()
  await expect(page.getByTestId('setting-future-setting')).toContainText('Future value')
  await chooseOption(page.getByLabel('Setting group', { exact: true }), 'Duels')
  await expect(page.getByTestId('setting-start-lp')).toBeVisible()
  await expect(page.getByTestId('setting-server-name')).toHaveCount(0)
  await page.getByRole('searchbox', { name: 'Find a setting', exact: true }).fill('not-a-setting')
  await expect(page.getByText('No matching settings.', { exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Clear filters', exact: true }).click()
  await expect(page.getByTestId('setting-future-setting')).toBeVisible()
  await page.getByLabel('Custom values only', { exact: true }).check()
  await expect(page.getByTestId('setting-future-setting')).toBeVisible()
  await expect(page.getByText('Default value', { exact: true })).toHaveCount(0)
  await page.getByLabel('Custom values only', { exact: true }).uncheck()
  await page
    .getByTestId('setting-start-lp')
    .getByRole('button', { name: 'Edit', exact: true })
    .click()
  await expect(page.getByRole('dialog')).toContainText('start-lp')
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
})

test('dark workspaces, errors and action dialogs stay legible and fit phones', async ({ page }) => {
  test.setTimeout(120_000)
  await page.emulateMedia({ colorScheme: 'dark' })
  await page.goto('/')
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await signIn(page)
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 900 })
    for (const path of [
      '/',
      '/accounts',
      '/accounts/1003',
      '/invites',
      '/news',
      '/moderation',
      '/audit',
      '/logs',
      '/backups',
      '/settings',
    ]) {
      await page.goto(path)
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
      await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
      if (path === '/settings' || path === '/accounts/1003') {
        await expect(
          page.locator(
            path === '/settings'
              ? '[data-testid="setting-start-lp"]'
              : '[data-testid="account-balance"]',
          ),
        ).toBeVisible()
        await page.screenshot({
          path: `../logs/dashboard-themes/${path === '/settings' ? 'settings' : 'account'}-dark-${width}.png`,
        })
      }
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
        path,
      ).toBeLessThanOrEqual(width)
      expect(
        (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
        path,
      ).toEqual([])
    }
  }
  await page
    .getByTestId('setting-start-lp')
    .getByRole('button', { name: 'Edit', exact: true })
    .click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await page.getByRole('dialog').getByLabel('Value', { exact: true }).fill('4000')
  await page.emulateMedia({ colorScheme: 'light' })
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light')
  await expect(page.getByRole('dialog').getByLabel('Value', { exact: true })).toHaveValue('4000')
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(page.getByRole('button', { name: 'Save change', exact: true })).toBeInViewport()
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
  await page.route('**/panel/api/v1/settings', (route) =>
    route.fulfill({
      status: 503,
      json: { code: 'persistence_failed', message: 'Could not read the settings. Try again.' },
    }),
  )
  await page.getByRole('button', { name: 'Refresh settings', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Could not read the settings.')
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})
