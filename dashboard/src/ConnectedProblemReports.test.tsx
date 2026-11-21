import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { vi, expect, test, afterEach } from 'vitest'
import { ConnectedProblemReports } from './ConnectedProblemReports'

afterEach(() => vi.unstubAllGlobals())

const row = {
  id: 'R-7K3Q-M2PA',
  at: 1_800_000_000,
  user_id: 7,
  account: 'Yugi',
  kind: 'network',
  launcher: '0.1.9',
  summary: 'the game lost the server',
  bytes: 17,
}
const json = (body: unknown) =>
  new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  })

test('lists reports, filters by account and opens one with its logs', async () => {
  const fetcher = vi.fn(async (url: string) =>
    url.includes('/problem-reports/R-7K3Q-M2PA')
      ? json({ ...row, server_log: ['server saw it'], logs: 'launcher line', text: 'all of it' })
      : json({ reports: [row], checked_at: new Date().toISOString() }),
  )
  vi.stubGlobal('fetch', fetcher)
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <ConnectedProblemReports />
    </QueryClientProvider>,
  )
  const user = userEvent.setup()
  expect(await screen.findByText('the game lost the server')).toBeVisible()
  await user.type(screen.getByLabelText('Account'), 'Yugi')
  await user.click(screen.getByRole('button', { name: 'Show' }))
  await waitFor(() =>
    expect(fetcher.mock.calls.some(([url]) => String(url).includes('who=Yugi'))).toBe(true),
  )
  await user.click(screen.getByRole('button', { name: 'Open report R-7K3Q-M2PA' }))
  const log = await screen.findByLabelText("The server's lines, then the launcher's log")
  expect(log).toHaveTextContent('server saw it')
  expect(log).toHaveTextContent('launcher line')
  expect(screen.getByRole('button', { name: 'Download' })).toBeEnabled()
})
