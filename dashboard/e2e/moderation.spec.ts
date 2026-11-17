import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import { ModerationResponse, StaffSession } from '../src/api/generated/validation/schemas'

async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const signedIn = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  StaffSession.parse(await (await signedIn).json())
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}
async function moderate(page: Page, who = 'TestModTarget') {
  await page.getByRole('button', { name: `View account ${who}`, exact: true }).click()
  await page.getByRole('button', { name: 'Moderate account', exact: true }).click()
}
test.describe.serial('Connected moderation', () => {
  test('mute survives a lost response and reload; lifting it is a new confirmed action', async ({
    page,
  }) => {
    await signIn(page)
    const ids: string[] = []
    const results: unknown[] = []
    await page.route('**/panel/api/v1/moderation', async (route) => {
      ids.push(route.request().headers()['idempotency-key'])
      const response = await route.fetch()
      results.push(ModerationResponse.parse(await response.json()))
      if (ids.length === 1) return route.abort('failed')
      return route.fulfill({ response })
    })
    await moderate(page)
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Action', { exact: true }).selectOption('mute')
    await dialog.getByLabel('Reason', { exact: true }).fill('Browser moderation check')
    await dialog.getByLabel('Duration (minutes)').fill('30')
    await expect(dialog.getByRole('button', { name: 'Apply action', exact: true })).toBeDisabled()
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(
      dialog.getByRole('button', { name: 'Retry same action', exact: true }),
    ).toBeVisible()
    await page.reload()
    await page.getByRole('button', { name: 'Resume moderation action', exact: true }).click()
    await dialog.getByRole('button', { name: 'Retry same action', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('Saved mute')
    expect(ids).toHaveLength(2)
    expect(ids[0]).toBe(ids[1])
    expect(results[0]).toEqual(results[1])
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await page.unroute('**/panel/api/v1/moderation')
    await moderate(page)
    await dialog.getByLabel('Action', { exact: true }).selectOption('unmute')
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('Saved unmute')
  })
  test('invalid durations can be corrected and uncertain disconnect outcomes never suggest a fresh retry', async ({
    page,
  }) => {
    await signIn(page)
    await moderate(page)
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Action', { exact: true }).selectOption('ban')
    await dialog.getByLabel('Duration (days)').fill('4294967295')
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(dialog.getByRole('alert')).toContainText('duration is too long')
    await dialog.getByRole('button', { name: 'Edit request', exact: true }).click()
    await dialog.getByLabel('Duration (days)').fill('1')
    const ids: string[] = []
    await page.route('**/panel/api/v1/moderation', async (route) => {
      ids.push(route.request().headers()['idempotency-key'])
      const response = await route.fetch()
      if (ids.length === 1) {
        const body = ModerationResponse.parse(await response.json())
        return route.fulfill({ response, json: { ...body, disconnect: 'pending' } })
      }
      return route.fulfill({ response })
    })
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(dialog.getByRole('alert')).toContainText(
      'account change is saved, but the disconnect outcome is unknown',
    )
    await dialog.getByRole('button', { name: 'Check outcome', exact: true }).click()
    await expect(
      dialog.getByText('The server ended this account’s game sessions.', { exact: false }),
    ).toBeVisible()
    expect(ids).toHaveLength(2)
    expect(ids[0]).toBe(ids[1])
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await page.unroute('**/panel/api/v1/moderation')
    await moderate(page)
    await dialog.getByLabel('Action', { exact: true }).selectOption('lift')
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('Saved lift')
  })

  test('moderators cannot act on another staff account and the confirmation fits a narrow screen', async ({
    page,
  }) => {
    await signIn(page, 'TestModerator')
    await page.getByRole('button', { name: 'View account TestModAdmin', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Moderate account', exact: true })).toBeDisabled()
    await page.getByRole('button', { name: 'Close', exact: true }).click()
    await page.setViewportSize({ width: 390, height: 844 })
    await moderate(page)
    await page.getByLabel('Action', { exact: true }).selectOption('ban')
    await expect(page.getByRole('dialog')).toContainText('disconnects their game')
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true)
  })
  test('sign out everywhere revokes your cookie but a lost-response retry cannot revoke your new sign-in', async ({
    page,
  }) => {
    await signIn(page, 'TestModAdmin')
    const ids: string[] = []
    await page.route('**/panel/api/v1/moderation', async (route) => {
      ids.push(route.request().headers()['idempotency-key'])
      const response = await route.fetch()
      if (ids.length === 1) return route.abort('failed')
      return route.fulfill({ response })
    })
    await moderate(page, 'TestModAdmin')
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Action', { exact: true }).selectOption('sign_out')
    await expect(dialog).toContainText('This is your account.')
    await dialog.getByLabel('Confirm this action for this account').check()
    await dialog.getByRole('button', { name: 'Apply action', exact: true }).click()
    await expect(
      dialog.getByRole('button', { name: 'Retry same action', exact: true }),
    ).toBeVisible()
    await signIn(page, 'TestModAdmin')
    await page.getByRole('button', { name: 'Resume moderation action', exact: true }).click()
    await dialog.getByRole('button', { name: 'Retry same action', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('Saved sign-out')
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    expect(ids).toHaveLength(2)
    expect(ids[0]).toBe(ids[1])
    await expect(page.getByRole('button', { name: 'Sign out', exact: true })).toBeVisible()
  })
})
