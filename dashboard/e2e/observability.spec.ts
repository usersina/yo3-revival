import { chooseOption } from './select'
import { test, expect } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { AuditPage, LogPage, StaffSession } from '../src/api/generated/validation/schemas'
async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  StaffSession.parse(await (await response).json())
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}
test('audit history reads real entries, pages by string cursor and distinguishes empty search from failure', async ({
  page,
}) => {
  await signIn(page)
  const audit = page.getByRole('region', { name: 'Audit log', exact: true })
  await chooseOption(audit.getByLabel('Entries per page'), '5')
  const first = AuditPage.parse(
    await (await page.request.get('/panel/api/v1/audit?limit=5')).json(),
  )
  await expect(audit.locator('tbody tr')).toHaveCount(5)
  await audit.getByRole('button', { name: 'Older entries', exact: true }).click()
  await expect(audit.locator(`tr[data-audit-id="${first.entries[0].id}"]`)).toHaveCount(0)
  await audit.getByRole('button', { name: 'Back to newest', exact: true }).click()
  await audit.getByLabel('Search audit history').fill('NoSuchAuditEntryEver')
  await audit.getByRole('button', { name: 'Search', exact: true }).click()
  await expect(audit.getByText('No audit entries match your search.')).toBeVisible()
  await page.route('**/panel/api/v1/audit?*', (route) => route.abort('failed'))
  await audit.getByRole('button', { name: 'Refresh audit history' }).click()
  await expect(audit.getByRole('alert')).toContainText('could not be refreshed')
})
test('moderators can read captured logs, filter problems and see fetch failures', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  const section = page.getByRole('region', { name: 'Server logs', exact: true })
  const value = LogPage.parse(await (await page.request.get('/panel/api/v1/logs?limit=200')).json())
  expect(value.lines.length).toBeGreaterThan(0)
  await expect(section.getByLabel('Recent server log lines')).toBeVisible()
  const changed = page.waitForResponse(
    (r) => r.url().includes('/panel/api/v1/logs?') && r.url().includes('problems=true'),
  )
  await section.getByLabel('Warnings and errors only').check()
  const problems = LogPage.parse(await (await changed).json())
  expect(problems.lines.every((line) => ['warn', 'error'].includes(line.level))).toBe(true)
  await page.route('**/panel/api/v1/logs?*', (route) => route.abort('failed'))
  await section.getByRole('button', { name: 'Refresh server logs' }).click()
  await expect(section.getByRole('alert')).toContainText('could not be refreshed')
})
test('audit details and captured logs stay accessible on a narrow viewport', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 850 })
  await signIn(page)
  const audit = page.getByRole('region', { name: 'Audit log', exact: true })
  await audit.locator('summary').first().click()
  await expect(audit.locator('details[open]')).toBeVisible()
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320)
})
