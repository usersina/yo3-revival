import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

test('card previews preserve the gift, show original text, and fit phones and dark mode', async ({
  page,
}) => {
  test.setTimeout(120_000) // Six viewport/theme checks share one real staff sign-in.
  await page.goto('/accounts')
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  await page.getByRole('link', { name: 'View account TestPlayer', exact: true }).click()
  await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
  const gift = page.getByRole('dialog', { name: 'Give a gift', exact: true })
  await gift.getByRole('radio').first().check()
  await gift.getByLabel('Copies', { exact: true }).fill('3')
  const preview = gift.getByRole('button', { name: /Preview.*Artwork 1/ })
  const fullRequests: string[] = []
  page.on('request', (request) => {
    if (new URL(request.url()).pathname.endsWith('/full')) fullRequests.push(request.url())
  })
  expect(fullRequests).toHaveLength(0)
  await preview.click()
  const dialog = page.getByRole('dialog', { name: 'Blue-Eyes White Dragon', exact: true })
  await expect(dialog).toBeVisible()
  await expect(dialog.getByRole('img')).toBeVisible()
  await expect
    .poll(() => dialog.getByRole('img').evaluate((el) => (el as HTMLImageElement).naturalWidth))
    .toBe(200)
  expect(fullRequests).toHaveLength(1)
  await expect(dialog.getByText('Original game text', { exact: true })).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(dialog).not.toBeVisible()
  await expect(preview).toBeFocused()
  await expect(gift.getByRole('radio').first()).toBeChecked()
  await expect(gift.getByLabel('Copies', { exact: true })).toHaveValue('3')
  await preview.click()
  await expect(dialog).toBeVisible()
  await expect
    .poll(() => dialog.getByRole('img').evaluate((el) => (el as HTMLImageElement).naturalWidth))
    .toBe(200)
  const transfers = await dialog
    .getByRole('img')
    .evaluate((image) =>
      performance
        .getEntriesByName((image as HTMLImageElement).src)
        .map((entry) => (entry as PerformanceResourceTiming).transferSize),
    )
  expect(fullRequests.length === 1 || transfers.includes(0)).toBe(true)
  await page.keyboard.press('Escape')
  await expect(dialog).not.toBeVisible()
  for (const theme of ['light', 'dark']) {
    await page.evaluate(
      (theme) => document.documentElement.setAttribute('data-theme', theme),
      theme,
    )
    for (const width of [320, 768, 1280]) {
      await page.setViewportSize({ width, height: 850 })
      await preview.click()
      await expect(dialog).toBeVisible()
      const box = await dialog.boundingBox()
      expect(box!.x).toBeGreaterThanOrEqual(0)
      expect(box!.x + box!.width).toBeLessThanOrEqual(width)
      expect(await dialog.evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true)
      expect(
        (await new AxeBuilder({ page }).include('.card-preview').analyze()).violations,
      ).toEqual([])
      await page.keyboard.press('Escape')
      await expect(dialog).not.toBeVisible()
    }
  }
})

test('a failed card image has a useful fallback without changing the gift', async ({ page }) => {
  await page.goto('/accounts')
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  await page.getByRole('link', { name: 'View account TestPlayer', exact: true }).click()
  await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
  const gift = page.getByRole('dialog', { name: 'Give a gift', exact: true })
  await gift.getByRole('radio').first().check()
  await page.route('**/panel/api/v1/cards/*/full?*', (route) =>
    route.fulfill({ status: 404, body: 'Missing image' }),
  )
  await gift.getByRole('button', { name: /Preview.*Artwork 1/ }).click()
  const dialog = page.getByRole('dialog', { name: 'Blue-Eyes White Dragon', exact: true })
  await expect(dialog.getByText('Image unavailable', { exact: true })).toBeVisible()
  await expect(dialog.getByText('Original game text', { exact: true })).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(gift.getByRole('radio').first()).toBeChecked()
  await gift.getByLabel('Confirm this gift for the selected player').check()
  await expect(gift.getByRole('button', { name: 'Give gift', exact: true })).toBeEnabled()
})
