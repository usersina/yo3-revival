import { chooseOption } from './select'
import { test, expect } from '@playwright/test'
import type { Page } from '@playwright/test'
import { randomUUID } from 'node:crypto'
import {
  AccountList,
  ServerOverview,
  StaffSession,
  SignInOutcome,
} from '../src/api/generated/validation/schemas'
async function signIn(page: Page, start = '/') {
  await page.goto(start)
  await page.getByLabel('User ID', { exact: true }).fill('TestAdmin')
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const signingIn = page.waitForResponse((response) =>
    response.url().endsWith('/panel/api/v1/sign-in'),
  )
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  SignInOutcome.parse(await (await signingIn).json())
  await expect(page.getByRole('button', { name: 'Sign out', exact: true })).toBeVisible()
}
test('overview shows checked operational metrics, backup failures and links online players to real account details', async ({
  page,
}) => {
  await signIn(page)
  const section = page.getByRole('region', { name: 'Live server', exact: true })
  await expect(section.getByText('Rules engines', { exact: true })).toBeVisible()
  await expect(section.getByText('Server memory', { exact: true })).toBeVisible()
  await expect(page.getByRole('region', { name: 'Recent problems', exact: true })).toBeVisible()
  const list = AccountList.parse(await (await page.request.get('/panel/api/v1/accounts')).json())
  const target = list.accounts.find((row) => row.login === 'TestOverview')!
  await page.route('**/panel/api/v1/server', async (route) => {
    const response = await route.fetch()
    const live = ServerOverview.parse(await response.json())
    const value = ServerOverview.parse({
      ...live,
      online: 1,
      backup: { kind: 'unavailable', message: 'Backup storage unavailable.' },
      online_players: [
        {
          player_id: target.players[0].player_id,
          user_id: target.user_id,
          name: 'TestOverview',
          duelling: true,
        },
      ],
    })
    return route.fulfill({ response, json: value })
  })
  await section.getByRole('button', { name: 'Refresh server status' }).click()
  await expect(section.getByRole('alert')).toContainText(
    'Backup check failed: Backup storage unavailable.',
  )
  await section.getByRole('link', { name: 'TestOverview', exact: true }).click()
  await expect(page).toHaveURL(`/accounts/${target.user_id}`)
  await expect(page.getByRole('heading', { name: 'TestOverview', level: 1 })).toBeVisible()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await page.reload()
  await expect(page.getByRole('heading', { name: 'TestOverview', level: 1 })).toBeVisible()
  await page.getByRole('link', { name: 'Back to accounts', exact: true }).click()
  await expect(page.getByRole('dialog')).toHaveCount(0)
})
test('moderation lists real saved standing and mute badges, with working filters', async ({
  page,
}) => {
  await signIn(page, '/moderation')
  const list = AccountList.parse(await (await page.request.get('/panel/api/v1/accounts')).json())
  const target = list.accounts.find((row) => row.login === 'TestOverview')!
  const session = StaffSession.parse(await (await page.request.get('/panel/api/v1/session')).json())
  const origin = new URL(page.url()).origin
  async function change(kind: 'ban' | 'mute' | 'lift' | 'unmute') {
    const response = await page.request.post('/panel/api/v1/moderation', {
      headers: {
        Origin: origin,
        'X-CSRF-Token': session.csrf_token,
        'Idempotency-Key': randomUUID(),
      },
      data: {
        kind,
        user_id: target.user_id,
        ...(['ban', 'mute'].includes(kind) ? { reason: 'Moderation list check' } : {}),
      },
    })
    expect(response.ok()).toBe(true)
  }
  try {
    await change('ban')
    await change('mute')
    await page.getByRole('button', { name: 'Refresh', exact: true }).click()
    await chooseOption(page.getByLabel('Standing', { exact: true }), 'banned')
    const row = page.getByRole('row').filter({ hasText: 'TestOverview' })
    await expect(row).toContainText('banned')
    await expect(row).toContainText('Muted')
    await expect(page.getByRole('row').filter({ hasText: 'TestPlayer' })).toHaveCount(0)
    await chooseOption(page.getByLabel('Standing', { exact: true }), 'muted')
    await expect(row).toBeVisible()
  } finally {
    await change('lift')
    await change('unmute')
  }
})
