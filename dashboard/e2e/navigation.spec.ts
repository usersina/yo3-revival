import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
async function signIn(page: Page, start = '/') {
  await page.goto(start)
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await response).status()).toBe(200)
}
const pages = [
  ['Accounts', '/accounts'],
  ['Invites & codes', '/invites'],
  ['News & maintenance', '/news'],
  ['Moderation', '/moderation'],
  ['Reports', '/reports'],
  ['Events', '/events'],
  ['Championship', '/championship'],
  ['Audit log', '/audit'],
  ['Server logs', '/logs'],
  ['Backups', '/backups'],
  ['Settings', '/settings'],
] as const

test('the default dashboard signs into real data and every navigation item has its own workspace', async ({
  page,
}) => {
  await signIn(page)
  await expect(page.getByRole('heading', { name: 'Overview', level: 1 })).toBeVisible()
  await expect(page.getByRole('region', { name: 'Live server', exact: true })).toBeVisible()
  await expect(page.getByText(/Sample data.*not connected/)).toHaveCount(0)
  for (const [label, path] of pages) {
    await page.getByRole('link', { name: label, exact: true }).click()
    await expect(page).toHaveURL(path)
    await expect(page.getByRole('heading', { name: label, level: 1 })).toBeVisible()
    if (path !== '/accounts' && path !== '/moderation')
      await expect(page.getByRole('region', { name: 'Accounts table', exact: true })).toHaveCount(0)
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
  }
  await page.reload()
  await expect(page.getByRole('heading', { name: 'Settings', level: 1 })).toBeVisible()
  await page.getByRole('button', { name: 'Sign out', exact: true }).click()
  await expect(page.getByLabel('User ID', { exact: true })).toBeVisible()
})
test('phone navigation closes after selection and real pages fit a narrow viewport', async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 850 })
  await signIn(page)
  for (const [label, path] of pages) {
    await page.getByRole('button', { name: 'Open navigation', exact: true }).click()
    const dialog = page.getByRole('dialog', { name: 'Navigation', exact: true })
    await dialog.getByRole('link', { name: label, exact: true }).click()
    await expect(dialog).not.toBeVisible()
    await expect(page.getByRole('heading', { name: label, level: 1 })).toBeVisible()
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
      path,
    ).toBeLessThanOrEqual(320)
    if (path === '/backups') {
      const box = await page
        .getByText(
          'Checked snapshots on this server. Copy them off the host to protect against losing the machine.',
          { exact: true },
        )
        .boundingBox()
      expect(box?.width).toBeGreaterThan(180)
    }
  }
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})
