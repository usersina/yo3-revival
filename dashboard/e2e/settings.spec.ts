import { expect, test } from '@playwright/test'
import { SettingsList, SettingsResult } from '../src/api/generated/validation/schemas'
import type { Page } from '@playwright/test'

async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}

test.describe.serial('Connected server settings', () => {
  test('settings use server metadata, save and reset with the generated contract', async ({
    page,
  }) => {
    await signIn(page)
    await page.getByText('All live settings', { exact: true }).click()
    const row = page.getByTestId('setting-headline')
    await row.getByRole('button', { name: 'Edit', exact: true }).click()
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Value', { exact: true }).fill('Browser-tested welcome')
    await dialog.getByLabel('Confirm this change').check()
    const saved = page.waitForResponse(
      (r) => r.url().endsWith('/panel/api/v1/settings') && r.request().method() === 'POST',
    )
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    expect(SettingsResult.parse(await (await saved).json()).message).toContain(
      'Browser-tested welcome',
    )
    await expect(row).toContainText('Browser-tested welcome')
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await row.getByRole('button', { name: 'Reset', exact: true }).click()
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('headline is now (none)')
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await expect(row.getByRole('button', { name: 'Reset', exact: true })).toBeDisabled()
  })

  test('lost maintenance response retains one request through reload without moving the countdown', async ({
    page,
  }) => {
    await signIn(page)
    const ids: string[] = []
    const bodies: unknown[] = []
    const results: unknown[] = []
    await page.route('**/panel/api/v1/settings', async (route) => {
      if (route.request().method() !== 'POST') return route.continue()
      ids.push(route.request().headers()['idempotency-key'])
      bodies.push(route.request().postDataJSON())
      const response = await route.fetch()
      results.push(SettingsResult.parse(await response.json()))
      if (ids.length === 1) return route.abort('failed')
      return route.fulfill({ response })
    })
    await page.getByRole('button', { name: 'Schedule maintenance', exact: true }).click()
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Starts in (minutes)').fill('60')
    await dialog.getByLabel('Message to players').fill('Browser maintenance check')
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('button', { name: 'Retry same change' })).toBeVisible()
    await page.reload()
    await page.getByRole('button', { name: 'Resume settings change' }).click()
    await dialog.getByRole('button', { name: 'Retry same change' }).click()
    await expect(dialog.getByRole('status')).toContainText('maintenance starts at')
    expect(ids).toHaveLength(2)
    expect(ids[0]).toBe(ids[1])
    expect(bodies[0]).toEqual(bodies[1])
    expect(results[0]).toEqual(results[1])
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await page.unroute('**/panel/api/v1/settings')
    const listed = page.waitForResponse(
      (r) => r.url().endsWith('/panel/api/v1/settings') && r.request().method() === 'GET',
    )
    await page.getByRole('button', { name: 'Refresh settings' }).click()
    const state = SettingsList.parse(await (await listed).json())
    expect(state.maintenance_at).toBe((results[0] as { maintenance_at: string }).maintenance_at)
    await page.getByRole('button', { name: 'Cancel maintenance', exact: true }).click()
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('status')).toHaveText('maintenance is off')
  })

  test('news is edited as announcements and saved settings survive a server validation error', async ({
    page,
  }) => {
    await signIn(page)
    await page.getByText('All live settings', { exact: true }).click()
    await page
      .getByTestId('setting-news')
      .getByRole('button', { name: 'Edit', exact: true })
      .click()
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('Title 1', { exact: true }).fill('Browser announcement')
    await dialog.getByLabel('Body 1', { exact: true }).fill('The real game news list.')
    await expect(dialog.getByRole('button', { name: 'Remove announcement 1' })).toBeDisabled()
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('Browser announcement')
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await page
      .getByTestId('setting-min-launcher')
      .getByRole('button', { name: 'Edit', exact: true })
      .click()
    await dialog.getByLabel('Value', { exact: true }).fill('invalid-version')
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('alert')).toContainText('version such as')
    await dialog.getByRole('button', { name: 'Edit request', exact: true }).click()
    await dialog.getByLabel('Value', { exact: true }).fill('')
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('min-launcher is now (none)')
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await page
      .getByTestId('setting-news')
      .getByRole('button', { name: 'Reset', exact: true })
      .click()
    await dialog.getByLabel('Confirm this change').check()
    await dialog.getByRole('button', { name: 'Save change', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('news is now')
  })

  test('unavailable settings show a recovery action and disable editing', async ({ page }) => {
    await page.route('**/panel/api/v1/settings', (route) =>
      route.fulfill({
        status: 503,
        contentType: 'application/json',
        body: JSON.stringify({ code: 'persistence_failed', message: 'Settings are unavailable.' }),
      }),
    )
    await signIn(page)
    await expect(
      page.getByText('Settings are unavailable. Use Refresh settings to try again.'),
    ).toBeVisible()
    await page.unroute('**/panel/api/v1/settings')
    await page.getByRole('button', { name: 'Refresh settings' }).click()
    await expect(
      page.getByRole('button', { name: 'Schedule maintenance', exact: true }),
    ).toBeEnabled()
    await page.setViewportSize({ width: 390, height: 844 })
    await page.getByRole('button', { name: 'Schedule maintenance', exact: true }).click()
    await expect(page.getByRole('dialog')).toBeVisible()
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true)
  })

  test('moderators can read settings but cannot change them', async ({ page }) => {
    await signIn(page, 'TestModerator')
    await expect(page.getByText('Only administrators can change server settings.')).toBeVisible()
    await page.getByText('All live settings', { exact: true }).click()
    await expect(
      page.getByTestId('setting-headline').getByRole('button', { name: 'Edit', exact: true }),
    ).toBeDisabled()
    await expect(
      page.getByRole('button', { name: 'Schedule maintenance', exact: true }),
    ).toBeDisabled()
  })
})
