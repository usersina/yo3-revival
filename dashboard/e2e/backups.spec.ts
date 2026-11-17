import { test, expect } from '@playwright/test'
import type { Page } from '@playwright/test'
import { readFile } from 'node:fs/promises'
import { CreateBackupResponse, StaffSession } from '../src/api/generated/validation/schemas'
async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  StaffSession.parse(await (await response).json())
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}
test('a lost creation response survives reload and the generated download client receives a real SQLite file', async ({
  page,
}) => {
  await signIn(page)
  let section = page.getByRole('region', { name: 'Database backups', exact: true })
  const ids: string[] = []
  let name = ''
  await page.route('**/panel/api/v1/backups', async (route) => {
    if (route.request().method() !== 'POST') return route.continue()
    ids.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    const value = CreateBackupResponse.parse(await response.json())
    name = value.result.name
    if (ids.length === 1) return route.abort('failed')
    return route.fulfill({ response })
  })
  await section.getByRole('button', { name: 'Create backup', exact: true }).click()
  await expect(section.getByRole('alert')).toContainText('original request is retained')
  await page.reload()
  section = page.getByRole('region', { name: 'Database backups', exact: true })
  await section.getByRole('button', { name: 'Check backup outcome', exact: true }).click()
  await expect(section.getByRole('status')).toContainText('Backup created.')
  expect(ids).toHaveLength(2)
  expect(ids[0]).toBe(ids[1])
  await section.getByRole('button', { name: `Download ${name}`, exact: true }).click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('password hashes')
  await expect(dialog.getByRole('button', { name: 'Download backup', exact: true })).toBeDisabled()
  await dialog.getByLabel('I will keep this backup private.').check()
  const downloading = page.waitForEvent('download')
  await dialog.getByRole('button', { name: 'Download backup', exact: true }).click()
  const file = await downloading
  expect(file.suggestedFilename()).toBe(`yo3-backup-${name}.db`)
  const path = await file.path()
  expect(path).not.toBeNull()
  expect((await readFile(path!)).subarray(0, 16).toString()).toBe('SQLite format 3\0')
  await expect(section.getByRole('status').filter({ hasText: 'Download started.' })).toBeVisible()
})
test('moderators can inspect snapshots but cannot create or download them; read errors are visible', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  const section = page.getByRole('region', { name: 'Database backups', exact: true })
  await expect(section.getByRole('button', { name: 'Create backup', exact: true })).toHaveCount(0)
  await expect(section.getByRole('button', { name: /^Download / })).toHaveCount(0)
  await page.route('**/panel/api/v1/backups', (route) => route.abort('failed'))
  await section.getByRole('button', { name: 'Refresh backups' }).click()
  await expect(section.getByRole('alert')).toContainText('last successfully loaded list')
})
test('an uncertain download keeps its ID across reload and shows reauthentication errors instead of downloading an error body', async ({
  page,
}) => {
  test.setTimeout(60000) // Sign-in, a snapshot and two page lifetimes share this test's budget.
  await signIn(page)
  const section = page.getByRole('region', { name: 'Database backups', exact: true })
  await section.getByRole('button', { name: 'Create backup', exact: true }).click()
  await expect(section.getByRole('status')).toContainText('Backup created.')
  const ids: string[] = []
  await page.route('**/panel/api/v1/backups/download', (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    return ids.length === 1
      ? route.abort('failed')
      : route.fulfill({
          status: 403,
          json: {
            code: 'reauthentication_required',
            message: 'Sign out and sign in again before downloading a backup.',
          },
        })
  })
  await section
    .getByRole('button', { name: /^Download / })
    .first()
    .click()
  await page.getByLabel('I will keep this backup private.').check()
  await page.getByRole('button', { name: 'Download backup', exact: true }).click()
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('response was lost')
  await page.reload()
  await page.getByRole('button', { name: 'Resume download', exact: true }).click()
  await page.getByLabel('I will keep this backup private.').check()
  await page.getByRole('button', { name: 'Retry same download', exact: true }).click()
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText(
    'Sign out and sign in again',
  )
  expect(ids).toHaveLength(2)
  expect(ids[0]).toBe(ids[1])
})

test('a proxy-disabled download is explained without offering a broken download button', async ({
  page,
}) => {
  await page.route('**/panel/api/v1/backups', async (route) => {
    if (route.request().method() !== 'GET') return route.continue()
    const response = await route.fetch()
    const data: unknown = await response.json()
    await route.fulfill({
      response,
      json: { ...(data as Record<string, unknown>), downloads_allowed: false },
    })
  })
  await signIn(page, 'TestAdmin')
  const section = page.getByRole('region', { name: 'Database backups', exact: true })
  await expect(
    section.getByText('Download copies through SSH. Public backup downloads are disabled.'),
  ).toBeVisible()
  await expect(section.getByRole('button', { name: /^Download / })).toHaveCount(0)
  await expect(section.getByRole('columnheader', { name: 'Download', exact: true })).toHaveCount(0)
})
