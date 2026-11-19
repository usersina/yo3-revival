import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { chooseOption } from './select'
import { HandleReportResult, ReportList } from '../src/api/generated/validation/schemas'

async function signIn(page: Page, user = 'TestAdmin', start = '/reports') {
  await page.goto(start)
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await response).status()).toBe(200)
}

const row = (page: Page, text: string) => page.getByRole('row').filter({ hasText: text })

test.describe.serial('Player reports', () => {
  test('open reports are listed, handled with a note and dismissed, and filtered by status', async ({
    page,
  }) => {
    const listed = page.waitForResponse(
      (r) => r.url().includes('/panel/api/v1/reports?status=open') && r.status() === 200,
    )
    await signIn(page)
    ReportList.parse(await (await listed).json())
    await expect(page.getByRole('heading', { name: 'Reports', level: 1 })).toBeVisible()
    await expect(page.getByRole('link', { name: 'Reports', exact: true })).toBeVisible()
    const table = page.getByRole('region', { name: 'Reports table', exact: true })
    await expect(table.getByText('Spams the chat in the lobby')).toBeVisible()
    await expect(table.getByText('Suspected cheating in an Auto Duel')).toBeVisible()
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])

    // Handle one with a note: it leaves the open list and shows the handler and note under Handled.
    const target = row(page, 'Spams the chat in the lobby')
    await target.getByRole('button', { name: /^Handle report \d+$/ }).click()
    const dialog = page.getByRole('dialog', { name: 'Handle report', exact: true })
    await dialog.getByLabel('Note', { exact: true }).fill('Warned in chat by the e2e check')
    const done = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/reports/handle'))
    await dialog.getByRole('button', { name: 'Mark handled', exact: true }).click()
    const result = HandleReportResult.parse(await (await done).json())
    expect(result.report.status).toBe('handled')
    expect(result.report.handled_by).toBe('TestAdmin')
    await expect(page.getByRole('status').filter({ hasText: /is handled/ })).toBeVisible()
    await expect(page.getByText('Spams the chat in the lobby')).toHaveCount(0)

    await chooseOption(page.getByLabel('Show', { exact: true }), 'handled')
    const handled = row(page, 'Spams the chat in the lobby')
    await expect(handled).toContainText('Handled')
    await expect(handled).toContainText('TestAdmin')
    await expect(handled).toContainText('Warned in chat by the e2e check')
    await expect(handled.getByRole('button')).toHaveCount(0)

    // Dismiss another one.
    await chooseOption(page.getByLabel('Show', { exact: true }), 'open')
    await row(page, 'Quit three duels in a row')
      .getByRole('button', { name: /^Dismiss report \d+$/ })
      .click()
    await page
      .getByRole('dialog', { name: 'Dismiss report', exact: true })
      .getByRole('button', { name: 'Dismiss report', exact: true })
      .click()
    await expect(page.getByText('Quit three duels in a row')).toHaveCount(0)
    await chooseOption(page.getByLabel('Show', { exact: true }), 'all')
    await expect(row(page, 'Quit three duels in a row')).toContainText('Dismissed')
    await expect(row(page, 'Spams the chat in the lobby')).toContainText('Handled')
    await expect(row(page, 'Insults in duels')).toContainText('Open')
  })

  test('the reporter and the reported player link to their account pages', async ({ page }) => {
    await signIn(page)
    const link = row(page, 'Insults in duels').getByRole('link', {
      name: 'View account of TestRoleTarget',
    })
    await expect(link).toHaveAttribute('href', /^\/accounts\/\d+$/)
    await link.click()
    await expect(
      page.getByRole('heading', { name: 'TestRoleTarget', level: 1, exact: true }),
    ).toBeVisible()
  })

  test('a moderator may handle reports too', async ({ page }) => {
    await signIn(page, 'TestModerator')
    const target = row(page, 'Suspected cheating in an Auto Duel')
    await target.getByRole('button', { name: /^Handle report \d+$/ }).click()
    await page
      .getByRole('dialog', { name: 'Handle report', exact: true })
      .getByRole('button', { name: 'Mark handled', exact: true })
      .click()
    await expect(page.getByText('Suspected cheating in an Auto Duel')).toHaveCount(0)
    await chooseOption(page.getByLabel('Show', { exact: true }), 'handled')
    await expect(row(page, 'Suspected cheating in an Auto Duel')).toContainText('TestModerator')
  })

  test('the account page shows the banlist and staff set it and put it back', async ({ page }) => {
    await signIn(page, 'TestAdmin', '/accounts')
    await page.getByRole('link', { name: 'View account TestPlayer', exact: true }).click()
    const region = page.getByRole('region', { name: 'Banlist', exact: true })
    await expect(region).toContainText('Season 16')
    await expect(region).toContainText("the server's default")
    await chooseOption(region.getByLabel('Banlist for next login'), '3')
    await region.getByRole('button', { name: 'Apply banlist', exact: true }).click()
    await expect(region.getByRole('status').filter({ hasText: 'Saved: Unlimited' })).toBeVisible()
    await expect(region).toContainText('chosen by the player or staff')
    await page.reload()
    await expect(page.getByRole('region', { name: 'Banlist', exact: true })).toContainText(
      'Unlimited',
    )
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
    const again = page.getByRole('region', { name: 'Banlist', exact: true })
    await chooseOption(again.getByLabel('Banlist for next login'), 'default')
    await again.getByRole('button', { name: 'Apply banlist', exact: true }).click()
    await expect(
      again.getByRole('status').filter({ hasText: "the server's default" }),
    ).toBeVisible()
    await expect(again).toContainText("(the server's default)")
  })
})
