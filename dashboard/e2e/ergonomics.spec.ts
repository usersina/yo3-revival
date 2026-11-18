import { expect, test } from '@playwright/test'
import type { Locator, Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

async function signIn(page: Page) {
  await page.goto('/accounts')
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const signingIn = page.waitForResponse((response) =>
    response.url().endsWith('/panel/api/v1/sign-in'),
  )
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await signingIn).status()).toBe(200)
  await expect(
    page.getByRole('button', { name: 'View account TestPlayer', exact: true }),
  ).toBeVisible()
}
async function insideViewport(control: Locator, width: number, height: number) {
  const box = await control.boundingBox()
  expect(box).not.toBeNull()
  expect(box!.x).toBeGreaterThanOrEqual(0)
  expect(box!.y).toBeGreaterThanOrEqual(0)
  expect(box!.x + box!.width).toBeLessThanOrEqual(width)
  expect(box!.y + box!.height).toBeLessThanOrEqual(height)
}

test('account actions and filters fit phones and tablets without sideways scrolling', async ({
  page,
}) => {
  await signIn(page)
  for (const width of [320, 390, 768, 1024]) {
    await page.setViewportSize({ width, height: 850 })
    await insideViewport(
      page.getByRole('button', { name: 'Create account', exact: true }),
      width,
      850,
    )
    await page.getByRole('searchbox', { name: 'Search accounts' }).fill('TestPlayer')
    await insideViewport(
      page.getByRole('button', { name: 'View account TestPlayer', exact: true }),
      width,
      850,
    )
    await insideViewport(
      page.getByRole('button', { name: 'Give BP to TestPlayer', exact: true }),
      width,
      850,
    )
    await page.getByRole('button', { name: 'Clear search', exact: true }).click()
    await expect(page.getByRole('searchbox', { name: 'Search accounts' })).toBeFocused()
  }
  await page.getByLabel('Standing', { exact: true }).selectOption('banned')
  await expect(page.getByText('No accounts match your filters.')).toBeVisible()
  await page.getByRole('button', { name: 'Clear filters', exact: true }).click()
  await expect(
    page.getByRole('button', { name: 'View account TestPlayer', exact: true }),
  ).toBeVisible()
})

test('long account dialogs retain their heading and actions on a short screen', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 520 })
  await signIn(page)
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'TestPlayer', exact: true })
  await insideViewport(dialog.getByRole('button', { name: 'Close', exact: true }), 390, 520)
  await dialog.locator('.modal-body').evaluate((el) => {
    el.scrollTop = el.scrollHeight
  })
  await insideViewport(dialog.getByRole('button', { name: 'Close dialog', exact: true }), 390, 520)
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  await expect(dialog).not.toBeVisible()
})

test('player actions fit the account dialog even on a wide desktop', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 })
  await signIn(page)
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  const players = page.getByRole('region', { name: 'Account players', exact: true })
  expect(await players.evaluate((el) => el.scrollWidth)).toBeLessThanOrEqual(
    await players.evaluate((el) => el.clientWidth),
  )
  await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Give a gift', exact: true })).toBeVisible()
})

test('card gifts identify artworks, link to official card details and validate copies before confirmation', async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 650 })
  await signIn(page)
  await page.getByRole('button', { name: 'View account TestPlayer', exact: true }).click()
  await page.getByRole('button', { name: 'Gift to TestPlayer', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Give a gift', exact: true })
  const artwork = dialog.getByRole('radio', {
    name: 'Blue-Eyes White Dragon · Artwork 2 · Konami 4007',
    exact: true,
  })
  await artwork.check()
  await expect(artwork).toBeChecked()
  const details = dialog.getByRole('link', {
    name: 'Card details (opens in a new tab)',
    exact: true,
  })
  await expect(details).toHaveAttribute(
    'href',
    'https://www.db.yugioh-card.com/yugiohdb/card_search.action?ope=2&cid=4007&request_locale=en',
  )
  await expect(details).toHaveAttribute('target', '_blank')
  await expect(dialog).toContainText('Artwork 2')
  await dialog.getByLabel('Confirm this gift for the selected player').check()
  await dialog.getByLabel('Copies', { exact: true }).fill('0')
  await expect(dialog.getByRole('button', { name: 'Give gift', exact: true })).toBeDisabled()
  await dialog.getByLabel('Copies', { exact: true }).fill('2')
  await dialog.getByLabel('Confirm this gift for the selected player').check()
  await expect(dialog.getByRole('button', { name: 'Give gift', exact: true })).toBeEnabled()
  await insideViewport(dialog.getByRole('button', { name: 'Give gift', exact: true }), 390, 650)
  await dialog.getByLabel('Find a gift', { exact: true }).fill('no such card')
  await expect(dialog.getByRole('radio')).toHaveCount(0)
  await expect(dialog.getByRole('link', { name: /Card details/ })).toHaveCount(0)
  await expect(dialog.getByRole('button', { name: 'Give gift', exact: true })).toBeDisabled()
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})
