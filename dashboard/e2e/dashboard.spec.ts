import { chooseOption } from './select'
import { expect, test } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

const pages = [
  ['/', 'Overview'],
  ['/accounts', 'Accounts'],
  ['/invites', 'Invites & codes'],
  ['/news', 'News & maintenance'],
  ['/moderation', 'Moderation'],
  ['/audit', 'Audit log'],
  ['/logs', 'Server logs'],
  ['/backups', 'Backups'],
  ['/settings', 'Settings'],
] as const

test('every page loads without browser errors or accessibility violations', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  for (const [path, name] of pages) {
    await page.goto(`/design${path}`)
    await expect(page.getByRole('heading', { name, level: 1 })).toBeVisible()
    await expect(page.getByText(/Sample data.*not connected/)).toBeVisible()
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()
    expect(
      results.violations,
      `${path}: ${JSON.stringify(results.violations.map((v) => ({ id: v.id, nodes: v.nodes.map((n) => n.target) })))}`,
    ).toEqual([])
  }
  expect(errors).toEqual([])
})

test('account changes, invites and audit entries work across navigation', async ({ page }) => {
  await page.goto('/design/accounts')
  await page.getByRole('button', { name: 'View YuseiFudo' }).click()
  let dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: 'Give BP' }).click()
  await dialog.getByLabel('BP amount').fill('750')
  await dialog.getByRole('button', { name: 'Add BP' }).click()
  await expect(dialog.getByText('4,950 BP')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('button', { name: 'View YuseiFudo' })).toBeFocused()
  await page.getByRole('link', { name: 'Audit log', exact: true }).click()
  await expect(page.getByText('Added 750 BP to YuseiFudo', { exact: true })).toBeVisible()
  await page.getByRole('link', { name: 'Invites & codes', exact: true }).click()
  await page.getByRole('button', { name: 'Create invite' }).click()
  dialog = page.getByRole('dialog')
  await dialog.getByLabel('Number of sign-ups').fill('8')
  await dialog.getByRole('button', { name: 'Create invite' }).click()
  await expect(page.getByText('8 sign-ups left')).toBeVisible()
})

test('phone navigation and tables stay within the viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 })
  await page.goto('/design/')
  await page.getByRole('button', { name: 'Open navigation' }).click()
  const menu = page.getByRole('dialog', { name: 'Navigation' })
  await menu.getByRole('link', { name: 'Accounts', exact: true }).click()
  await expect(menu).not.toBeVisible()
  await expect(page.getByRole('heading', { name: 'Accounts', level: 1 })).toBeVisible()
  await page.getByRole('button', { name: 'View YuseiFudo' }).click()
  await expect(page.getByRole('dialog', { name: 'YuseiFudo' })).toBeVisible()
  await page.keyboard.press('Escape')
  for (const width of [320, 390]) {
    await page.setViewportSize({ width, height: 844 })
    for (const [path] of pages) {
      await page.goto(`/design${path}`)
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth > window.innerWidth,
      )
      expect(overflow, `${path} overflows a ${width}px phone viewport`).toBe(false)
    }
  }
})

test('moderator preview cannot use admin actions', async ({ page }) => {
  await page.goto('/design/settings')
  await chooseOption(page.getByLabel('Preview role'), 'moderator')
  await expect(page.getByRole('button', { name: 'Save changes' })).toBeDisabled()
  await expect(page.getByLabel('Server name')).toBeDisabled()
  await page.getByRole('link', { name: 'News & maintenance', exact: true }).click()
  await expect(page.getByRole('button', { name: 'New announcement' })).toHaveCount(0)
  await page.getByRole('link', { name: 'Backups', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Create backup' })).toHaveCount(0)
})

test('page navigation returns to the top without stealing account dialog focus', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 720 })
  await page.goto('/design/settings')
  await page.getByRole('button', { name: 'Save changes' }).scrollIntoViewIfNeeded()
  expect(await page.evaluate(() => window.scrollY)).toBeGreaterThan(0)
  await page.getByRole('link', { name: 'Accounts', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(0)
  await page.goto('/design/accounts?account=1')
  await expect(page.getByRole('dialog', { name: 'YuseiFudo' })).toBeVisible()
  await expect
    .poll(() =>
      page.getByRole('dialog').evaluate((dialog) => dialog.contains(document.activeElement)),
    )
    .toBe(true)
})
