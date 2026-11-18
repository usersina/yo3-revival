import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { vi, expect, test, afterEach } from 'vitest'
import { DashboardUpdate, PageLoadBoundary } from './DashboardVersion'

afterEach(() => vi.unstubAllGlobals())

test('a newer build offers reload without reloading or discarding an open form', async () => {
  const fetcher = vi
    .fn()
    .mockResolvedValue({ ok: true, json: async () => ({ commit: 'b'.repeat(40) }) })
  vi.stubGlobal('fetch', fetcher)
  render(
    <>
      <DashboardUpdate />
      <input aria-label="Draft" defaultValue="unsaved" />
    </>,
  )
  await screen.findByText('An update is available.')
  expect(screen.getByRole('button', { name: 'Reload dashboard' })).toBeVisible()
  expect(screen.getByLabelText('Draft')).toHaveValue('unsaved')
  expect(fetcher.mock.calls[0][0]).toMatch(/^\/assets\/dashboard-version.json\?check=/)
  expect(fetcher.mock.calls[0][1]).toMatchObject({ cache: 'no-store', credentials: 'same-origin' })
})

test('network failures and invalid metadata stay quiet and a focus check can recover', async () => {
  const fetcher = vi.fn().mockRejectedValue(new Error('offline'))
  vi.stubGlobal('fetch', fetcher)
  render(<DashboardUpdate />)
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(1))
  expect(screen.queryByRole('status')).toBeNull()
  fetcher.mockResolvedValue({ ok: true, json: async () => ({ commit: 'invalid' }) })
  await act(async () => fireEvent.focus(window))
  expect(screen.queryByRole('status')).toBeNull()
  fetcher.mockResolvedValue({ ok: true, json: async () => ({ commit: 'c'.repeat(40) }) })
  await act(async () => fireEvent.focus(window))
  expect(screen.getByText('An update is available.')).toBeVisible()
})

test('missing lazy assets offer recovery instead of an uncaught error or automatic reload', () => {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
  render(<DashboardUpdate />)
  const event = new Event('vite:preloadError', { cancelable: true })
  act(() => {
    window.dispatchEvent(event)
  })
  expect(event.defaultPrevented).toBe(true)
  expect(screen.getByText('This page needs the latest dashboard files.')).toBeVisible()
})

test('a failed lazy page still displays a recoverable screen', () => {
  const error = vi.spyOn(console, 'error').mockImplementation(() => {})
  function Broken(): never {
    throw new Error('Failed to fetch dynamically imported module')
  }
  render(
    <PageLoadBoundary>
      <Broken />
    </PageLoadBoundary>,
  )
  expect(screen.getByRole('heading', { name: 'This page could not load' })).toBeVisible()
  expect(screen.getByRole('button', { name: 'Reload dashboard' })).toBeVisible()
  error.mockRestore()
})
