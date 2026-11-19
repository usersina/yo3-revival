import { test, expect } from '@playwright/test'
import type { Page } from '@playwright/test'
import {
  ServerOverview,
  StopResponse,
  SignInOutcome,
} from '../src/api/generated/validation/schemas'
async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  SignInOutcome.parse(await (await response).json())
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}
test('live status shows real data and does not promise to stop the disposable API', async ({
  page,
}) => {
  await signIn(page)
  const section = page.getByRole('region', { name: 'Live server', exact: true })
  await expect(section.getByText('Version', { exact: true })).toBeVisible()
  const status = ServerOverview.parse(await (await page.request.get('/panel/api/v1/server')).json())
  expect(status.accounts).toBeGreaterThan(0)
  expect(status.can_stop).toBe(false)
  await expect(section.getByRole('button', { name: 'Stop server…', exact: true })).toBeDisabled()
  await page.route('**/panel/api/v1/server', (route) => route.abort('failed'))
  await section.getByRole('button', { name: 'Refresh server status' }).click()
  await expect(section.getByRole('alert')).toContainText('could not be refreshed')
})
test('stop confirmation retains run identity and request ID across an uncertain response and reload', async ({
  page,
}) => {
  // Fixture-only stopping capability: stopping the shared E2E service would invalidate every other browser test.
  await page.route('**/panel/api/v1/server', async (route) => {
    const response = await route.fetch()
    const value = ServerOverview.parse(await response.json())
    return route.fulfill({ response, json: { ...value, can_stop: true } })
  })
  await signIn(page)
  const ids: string[] = []
  const runs: string[] = []
  await page.route('**/panel/api/v1/server/stop', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    const data = route.request().postDataJSON() as { run_id: string }
    runs.push(data.run_id)
    if (ids.length === 1) return route.abort('failed')
    const value = StopResponse.parse({
      result: { run_id: data.run_id, request_id: ids[0], message: 'Graceful stop requested.' },
      stop: 'completed',
    })
    return route.fulfill({ status: 200, json: value })
  })
  await page.getByRole('button', { name: 'Stop server…', exact: true }).click()
  let dialog = page.getByRole('dialog')
  await expect(dialog.getByRole('button', { name: 'Stop server', exact: true })).toBeDisabled()
  await dialog.getByLabel('I understand all connected games will disconnect.').check()
  await dialog.getByRole('button', { name: 'Stop server', exact: true }).click()
  await expect(dialog.getByRole('alert')).toContainText('could not be confirmed')
  await page.reload()
  await page.getByRole('button', { name: 'Resume stop request', exact: true }).click()
  dialog = page.getByRole('dialog')
  await dialog.getByRole('button', { name: 'Retry same stop', exact: true }).click()
  await expect(dialog.getByRole('status')).toContainText('Graceful stop requested')
  expect(ids).toHaveLength(2)
  expect(ids[0]).toBe(ids[1])
  expect(runs[0]).toBe(runs[1])
  await dialog.getByRole('button', { name: 'Done', exact: true }).click()
})
test('moderators cannot stop the server', async ({ page }) => {
  await signIn(page, 'TestModerator')
  await expect(page.getByRole('button', { name: 'Stop server…', exact: true })).toHaveCount(0)
})
