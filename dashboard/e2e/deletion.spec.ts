import { test, expect } from '@playwright/test'
import type { Page } from '@playwright/test'
import { DeleteAccountResponse, SignInOutcome } from '../src/api/generated/validation/schemas'
async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/connected')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  SignInOutcome.parse(await (await response).json())
  await expect(page.getByRole('heading', { name: 'Accounts', exact: true })).toBeVisible()
}
test.describe.serial('Connected account deletion', () => {
  test('requires typed confirmation and recovers a lost response after reload', async ({
    page,
  }) => {
    await signIn(page)
    await page.getByRole('button', { name: 'View account TestDeleteTarget', exact: true }).click()
    await page.getByRole('button', { name: 'Delete account…', exact: true }).click()
    const dialog = page.getByRole('dialog')
    const heading = await dialog.getByRole('heading').innerText()
    const number = heading.match(/#(\d+)/)?.[1]
    expect(number).toBeTruthy()
    const submit = dialog.getByRole('button', { name: 'Delete account permanently', exact: true })
    await expect(submit).toBeDisabled()
    await dialog.getByLabel(`Type ${number} to confirm deletion`).fill('wrong')
    await expect(submit).toBeDisabled()
    await dialog.getByLabel(`Type ${number} to confirm deletion`).fill(number!)
    await expect(submit).toBeEnabled()
    const ids: string[] = []
    const results: unknown[] = []
    await page.route('**/panel/api/v1/accounts/delete', async (route) => {
      ids.push(route.request().headers()['idempotency-key'])
      const response = await route.fetch()
      results.push(DeleteAccountResponse.parse(await response.json()))
      if (ids.length === 1) return route.abort('failed')
      return route.fulfill({ response })
    })
    await submit.click()
    await expect(dialog.getByRole('alert')).toContainText('Could not confirm deletion')
    await page.reload()
    await page.getByRole('button', { name: 'Resume account deletion', exact: true }).click()
    await dialog.getByRole('button', { name: 'Retry same deletion', exact: true }).click()
    await expect(dialog.getByRole('status')).toContainText('was deleted')
    expect(ids).toHaveLength(2)
    expect(ids[0]).toBe(ids[1])
    expect(results[0]).toEqual(results[1])
    await dialog.getByRole('button', { name: 'Done', exact: true }).click()
    await expect(
      page.getByRole('button', { name: 'View account TestDeleteTarget', exact: true }),
    ).toHaveCount(0)
    expect(
      await page.evaluate(() =>
        Object.keys(sessionStorage).filter((k) => k.startsWith('yo3:pending-deletion:')),
      ),
    ).toEqual([])
  })
  test('moderators cannot start deletion', async ({ page }) => {
    await signIn(page, 'TestModerator')
    await page.getByRole('button', { name: 'View account TestDeleteAdmin', exact: true }).click()
    await expect(page.getByRole('button', { name: 'Delete account…', exact: true })).toHaveCount(0)
  })
  test('storage failures refuse to send a destructive request', async ({ page }) => {
    await signIn(page)
    await page.getByRole('button', { name: 'View account TestDeleteAdmin', exact: true }).click()
    await page.getByRole('button', { name: 'Delete account…', exact: true }).click()
    const dialog = page.getByRole('dialog')
    const number = (await dialog.getByRole('heading').innerText()).match(/#(\d+)/)?.[1]
    await dialog.getByLabel(`Type ${number} to confirm deletion`).fill(number!)
    let sent = 0
    page.on('request', (r) => {
      if (r.url().endsWith('/panel/api/v1/accounts/delete')) sent++
    })
    await page.evaluate(() => {
      Storage.prototype.setItem = () => {
        throw new Error('storage disabled')
      }
    })
    await dialog.getByRole('button', { name: 'Delete account permanently', exact: true }).click()
    await expect(
      page.getByRole('alert').filter({ hasText: 'Could not retain this request safely' }),
    ).toBeVisible()
    expect(sent).toBe(0)
  })
})
