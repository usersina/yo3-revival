import type { Locator } from '@playwright/test'

/** Select through the actual popup, including its keyboard/focus/portal implementation. */
export async function chooseOption(control: Locator, value: string) {
  await control.click()
  await control
    .page()
    .locator(`[role="option"][data-value=${JSON.stringify(value)}]`)
    .click()
}
