import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'
import { EventList, EventResult } from '../src/api/generated/validation/schemas'

async function signIn(page: Page, user = 'TestAdmin', start = '/events') {
  await page.goto(start)
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await response).status()).toBe(200)
}

const row = (page: Page, text: string) => page.getByRole('row').filter({ hasText: text })

test.describe.serial('Official events', () => {
  test('the seeded event is listed with its entrants and best scores', async ({ page }) => {
    const listed = page.waitForResponse(
      (r) => r.url().endsWith('/panel/api/v1/events') && r.status() === 200,
    )
    await signIn(page)
    const list = EventList.parse(await (await listed).json())
    expect(list.events.some((e) => e.name === 'Seeded Gunslinger')).toBe(true)
    await expect(page.getByRole('heading', { name: 'Events', level: 1 })).toBeVisible()
    const seeded = row(page, 'Seeded Gunslinger')
    await expect(seeded).toContainText('Gunslinger')
    await expect(seeded).toContainText('Running')
    await expect(seeded).toContainText('counts win streak')
    await expect(seeded).toContainText('100 BP for taking part')
    await expect(seeded).toContainText('400 BP at 3 win streak')
    await expect(seeded.getByRole('cell', { name: '2', exact: true })).toBeVisible()
    await expect(seeded).toContainText('TestPlayer 3')
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
  })

  test('an admin schedules, edits and cancels an event', async ({ page }) => {
    await signIn(page)
    await page.getByRole('button', { name: 'Schedule event', exact: true }).click()
    const dialog = page.getByRole('dialog', { name: 'Schedule event', exact: true })
    await dialog.getByLabel('Name', { exact: true }).fill('E2E Duelist Trial')
    await dialog.getByLabel('BP for taking part', { exact: true }).fill('150')
    await dialog.getByLabel('Score that earns more (wins)').fill('5')
    await dialog.getByLabel('BP for reaching it', { exact: true }).fill('500')
    expect(
      (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
    ).toEqual([])
    const created = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/events/create'))
    await dialog.getByRole('button', { name: 'Schedule event', exact: true }).click()
    const result = EventResult.parse(await (await created).json())
    expect(result.event.kind).toBe('duelist-trial')
    expect(result.event.prize).toEqual({ participation_bp: 150, threshold: 5, threshold_bp: 500 })
    await expect(page.getByRole('status').filter({ hasText: /is scheduled from/ })).toBeVisible()
    const mine = row(page, 'E2E Duelist Trial')
    await expect(mine).toContainText('Duelist Trial')
    await expect(mine).toContainText('150 BP for taking part · 500 BP at 5 wins')
    await expect(mine).toContainText('None yet')

    // Edit: the kind stays, the prize changes.
    await mine.getByRole('button', { name: /^Edit event \d+$/ }).click()
    const editing = page.getByRole('dialog', { name: 'Edit event', exact: true })
    await expect(editing.getByLabel('Kind', { exact: true })).toBeDisabled()
    await editing.getByLabel('BP for taking part', { exact: true }).fill('250')
    const edited = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/events/edit'))
    await editing.getByRole('button', { name: 'Save changes', exact: true }).click()
    expect(EventResult.parse(await (await edited).json()).event.prize.participation_bp).toBe(250)
    await expect(row(page, 'E2E Duelist Trial')).toContainText('250 BP for taking part')

    // A bad time range is refused with the server's reason.
    await row(page, 'E2E Duelist Trial')
      .getByRole('button', { name: /^Edit event \d+$/ })
      .click()
    const again = page.getByRole('dialog', { name: 'Edit event', exact: true })
    await again.getByLabel('Ends', { exact: true }).fill('2020-01-01T00:00')
    await again.getByRole('button', { name: 'Save changes', exact: true }).click()
    await expect(again.getByRole('alert')).toContainText('must end after it starts')
    await again.getByRole('button', { name: 'Close', exact: true }).click()

    // Cancel: it stays in the staff's list as cancelled, with no actions.
    await row(page, 'E2E Duelist Trial')
      .getByRole('button', { name: /^Cancel event \d+$/ })
      .click()
    const confirm = page.getByRole('dialog', { name: 'Cancel event', exact: true })
    const cancelled = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/events/cancel'))
    await confirm.getByRole('button', { name: 'Cancel event', exact: true }).click()
    expect(EventResult.parse(await (await cancelled).json()).event.phase).toBe('cancelled')
    await expect(row(page, 'E2E Duelist Trial')).toContainText('Cancelled')
    await expect(row(page, 'E2E Duelist Trial').getByRole('button')).toHaveCount(0)
  })

  test('a moderator sees the events and cannot change them', async ({ page }) => {
    await signIn(page, 'TestModerator')
    await expect(row(page, 'Seeded Gunslinger')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Schedule event', exact: true })).toBeDisabled()
    await expect(
      row(page, 'Seeded Gunslinger').getByRole('button', { name: /^Edit event \d+$/ }),
    ).toBeDisabled()
  })
})
