import { expect, test } from '@playwright/test'
import type { Page } from '@playwright/test'
import AxeBuilder from '@axe-core/playwright'

async function signIn(page: Page, user = 'TestAdmin') {
  await page.goto('/accounts')
  await page.getByLabel('User ID', { exact: true }).fill(user)
  await page.getByLabel('Password', { exact: true }).fill('DashboardDemo123!')
  const response = page.waitForResponse((r) => r.url().endsWith('/panel/api/v1/sign-in'))
  await page.getByRole('button', { name: 'Sign in', exact: true }).click()
  expect((await response).status()).toBe(200)
}

test('an account opens a full workspace with a durable URL and contextual actions', async ({
  page,
}) => {
  await signIn(page)
  const link = page.getByRole('link', { name: 'View account TestOverview', exact: true })
  await expect(link).toHaveAttribute('href', /^\/accounts\/\d+$/)
  await link.click()
  const url = page.url()
  await expect(
    page.getByRole('heading', { name: 'TestOverview', level: 1, exact: true }),
  ).toBeVisible()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  for (const name of ['Players', 'Account access', 'Profile', 'Moderation', 'Delete account'])
    await expect(page.getByRole('region', { name, exact: true })).toBeVisible()
  await page.reload()
  await expect(
    page.getByRole('heading', { name: 'TestOverview', level: 1, exact: true }),
  ).toBeVisible()
  const balance = page.getByTestId('account-balance')
  const before = Number((await balance.innerText()).replace(/[^0-9]/g, ''))
  await page.getByRole('button', { name: 'Give BP', exact: true }).click()
  await page.getByLabel('BP amount').fill('7')
  await page.getByRole('button', { name: 'Add BP', exact: true }).click()
  await expect(balance).toHaveText(`${(before + 7).toLocaleString()} BP`)
  await expect(page).toHaveURL(url)
  await page.getByRole('button', { name: 'Gift to TestOverview', exact: true }).click()
  await expect(page.getByRole('dialog', { name: 'Give a gift', exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
  await expect(page).toHaveURL(url)
  await expect(
    page.getByRole('heading', { name: 'TestOverview', level: 1, exact: true }),
  ).toBeVisible()
  await page.getByRole('link', { name: 'Back to accounts', exact: true }).click()
  await expect(page).toHaveURL('/accounts')
  await page.goBack()
  await expect(page).toHaveURL(url)
  expect(
    (await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze()).violations,
  ).toEqual([])
})

test('account sections and edit confirmations work on phones, tablets and desktop', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('link', { name: 'View account TestPlayer', exact: true }).click()
  for (const width of [320, 390, 768, 1440]) {
    await page.setViewportSize({ width, height: 650 })
    await expect(
      page.getByRole('heading', { name: 'TestPlayer', level: 1, exact: true }),
    ).toBeVisible()
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      width,
    )
    await page.getByRole('button', { name: 'Change country', exact: true }).click()
    const dialog = page.getByRole('dialog', { name: 'Change country', exact: true })
    await expect(dialog.getByLabel('Country', { exact: true })).toBeVisible()
    await expect(dialog.getByRole('button', { name: 'Save country', exact: true })).toBeInViewport()
    await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
    await expect(page.getByRole('dialog')).toHaveCount(0)
  }
})

test('direct account links show permissions and missing accounts without stale controls', async ({
  page,
}) => {
  await signIn(page, 'TestModerator')
  await page.getByRole('link', { name: 'View account TestAdmin', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Give BP', exact: true })).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Change role', exact: true })).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Moderate account', exact: true })).toBeDisabled()
  await expect(page.getByRole('button', { name: 'Delete account…', exact: true })).toHaveCount(0)
  await page.goto('/accounts/4294967295')
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Give BP', exact: true })).toHaveCount(0)
  await expect(page.getByRole('link', { name: 'Back to accounts', exact: true })).toBeVisible()
  await page.goto('/accounts/not-an-id')
  await expect(page.getByRole('alert')).toContainText('invalid')
  await expect(page.getByRole('button', { name: 'Give BP', exact: true })).toHaveCount(0)
})

test('every account action keeps its target and returns to the full page', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 650 })
  await signIn(page)
  await page.getByRole('link', { name: 'View account TestPlayer', exact: true }).click()
  const url = page.url()
  for (const [button, title] of [
    ['Change role', 'Change role'],
    ['Add player', 'Add player'],
    ['Rename TestPlayer', 'Rename player'],
    ['Moderate account', 'Moderate TestPlayer'],
    ['Reveal key', 'Reveal account key'],
    ['Reset password', 'Reset password'],
  ]) {
    await page.getByRole('button', { name: button, exact: true }).click()
    await expect(page.getByRole('dialog', { name: title, exact: true })).toBeVisible()
    await expect(page).toHaveURL(url)
    await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
    await expect(page.getByRole('dialog')).toHaveCount(0)
    await expect(
      page.getByRole('heading', { name: 'TestPlayer', level: 1, exact: true }),
    ).toBeVisible()
  }
  await page.getByRole('button', { name: 'Delete account…', exact: true }).click()
  const deletion = page.getByRole('dialog', { name: /^Delete account #/ })
  await expect(deletion).toContainText('TestPlayer')
  await expect(
    deletion.getByRole('button', { name: 'Delete account permanently', exact: true }),
  ).toBeDisabled()
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
  await expect(page).toHaveURL(url)
})

test('an uncertain account-page grant survives reload with the original ID and refreshes the balance once', async ({
  page,
}) => {
  await signIn(page)
  await page.getByRole('link', { name: 'View account TestRoleTarget', exact: true }).click()
  const url = page.url()
  const balance = page.getByTestId('account-balance')
  const before = Number((await balance.innerText()).replace(/[^0-9]/g, ''))
  const ids: string[] = []
  await page.route('**/panel/api/v1/give-bp', async (route) => {
    ids.push(route.request().headers()['idempotency-key'])
    const response = await route.fetch()
    return ids.length === 1 ? route.abort('failed') : route.fulfill({ response })
  })
  await page.getByRole('button', { name: 'Give BP', exact: true }).click()
  await page.getByLabel('BP amount').fill('3')
  await page.getByRole('button', { name: 'Add BP', exact: true }).click()
  await expect(page.getByRole('dialog').getByRole('alert')).toContainText('could not confirm')
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Give BP', exact: true })).toBeDisabled()
  await page.reload()
  await page.getByRole('button', { name: 'Resume grant', exact: true }).click()
  await page.getByRole('button', { name: 'Retry same grant', exact: true }).click()
  await expect(balance).toHaveText(`${(before + 3).toLocaleString()} BP`)
  await expect(page.getByRole('dialog')).toHaveCount(0)
  expect(ids).toHaveLength(2)
  expect(ids[0]).toBe(ids[1])
  await expect(page).toHaveURL(url)
})
