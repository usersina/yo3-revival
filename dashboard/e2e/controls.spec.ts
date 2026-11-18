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

test('groups collapse, searching reveals hidden settings and selects support the keyboard', async ({
  page,
}) => {
  await signIn(page)
  const server = page.getByRole('button', { name: /Server & access/ })
  await expect(server).toHaveAttribute('aria-expanded', 'true')
  await server.click()
  await expect(page.getByTestId('setting-server-name')).toBeHidden()
  await page.getByRole('searchbox', { name: 'Find a setting', exact: true }).fill('server-name')
  await expect(page.getByTestId('setting-server-name')).toBeVisible()
  await page.getByRole('button', { name: 'Clear search', exact: true }).click()
  await expect(server).toHaveAttribute('aria-expanded', 'false')
  const theme = page.getByRole('combobox', { name: 'Color theme', exact: true })
  await theme.focus()
  await theme.press('Enter')
  await expect(page.getByRole('listbox')).toBeVisible()
  await page.keyboard.press('End')
  await page.keyboard.press('Enter')
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await expect(theme).toBeFocused()
})

test('invalid settings show linked inline errors and clear after correction', async ({ page }) => {
  await signIn(page)
  await page.getByRole('searchbox', { name: 'Find a setting', exact: true }).fill('start-lp')
  await page
    .getByTestId('setting-start-lp')
    .getByRole('button', { name: 'Edit', exact: true })
    .click()
  const dialog = page.getByRole('dialog')
  const value = dialog.getByLabel('Value', { exact: true })
  await value.fill('')
  await value.blur()
  await expect(value).toHaveAttribute('aria-invalid', 'true')
  await expect(value).toHaveAccessibleDescription(/Enter a value/)
  await expect(dialog.getByText('Enter a value.', { exact: true })).toBeVisible()
  await value.fill('100.5')
  await expect(value).toHaveAccessibleDescription(/whole number/)
  await value.fill('8000')
  await expect(value).not.toHaveAttribute('aria-invalid', 'true')
  await value.fill('100000')
  await dialog.getByLabel('Confirm this change').check()
  let posted = false
  page.on('request', (request) => {
    if (request.method() === 'POST' && request.url().endsWith('/settings')) posted = true
  })
  await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
  await expect(value).toBeFocused()
  await expect(value).toHaveAccessibleDescription('Enter 99999 or less.')
  expect(posted).toBe(false)
  await page.screenshot({ path: '../logs/dashboard-controls/invalid-setting.png' })
})

test('open dropdowns fit phones and desktops in both themes', async ({ page }) => {
  await signIn(page)
  for (const theme of ['light', 'dark']) {
    for (const width of [390, 1440]) {
      await page.setViewportSize({ width, height: 900 })
      await page.getByRole('combobox', { name: 'Color theme', exact: true }).click()
      await page
        .getByRole('option', { name: theme === 'dark' ? 'Dark' : 'Light', exact: true })
        .click()
      await page.getByRole('combobox', { name: 'Setting group', exact: true }).click()
      const menu = page.getByRole('listbox')
      await expect(menu).toBeVisible()
      const box = await menu.boundingBox()
      expect(box).not.toBeNull()
      expect(box!.x).toBeGreaterThanOrEqual(0)
      expect(box!.x + box!.width).toBeLessThanOrEqual(width)
      expect(
        (
          await new AxeBuilder({ page })
            .include('[data-slot="select-content"]')
            .withTags(['wcag2a', 'wcag2aa'])
            .analyze()
        ).violations,
      ).toEqual([])
      await page.screenshot({ path: `../logs/dashboard-controls/settings-${theme}-${width}.png` })
      await page.keyboard.press('Tab')
      await expect(menu).toBeVisible()
      await expect(menu).toContainText(await page.locator(':focus').innerText())
      await page.keyboard.press('Escape')
      expect(
        (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
      ).toEqual([])
    }
  }
})
