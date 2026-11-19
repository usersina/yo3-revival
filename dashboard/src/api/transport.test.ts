import { afterEach, describe, expect, it, vi } from 'vitest'
import { giveBp, getStaffSession } from './generated/http/client'
import { ApiError, GiveBpResult, StaffSession } from './generated/validation/schemas'
import { ApiHttpError, apiFetch, setCsrfToken } from './transport'

const result = GiveBpResult.parse({
  user_id: 7,
  bp: 5100,
  message: 'Granted.',
  request_id: 'test-grant-00000001',
})

afterEach(() => {
  vi.unstubAllGlobals()
  setCsrfToken(undefined)
})

describe('generated API transport', () => {
  it('returns card PNGs as blobs using normal browser HTTP caching', async () => {
    const fetch = vi
      .fn()
      .mockResolvedValue(new Response('png-bytes', { headers: { 'Content-Type': 'image/png' } }))
    vi.stubGlobal('fetch', fetch)
    const image = await apiFetch<Blob>('/panel/api/v1/cards/817/full?v=revision')
    expect(await image.text()).toBe('png-bytes')
    expect(fetch.mock.calls[0][1]).toMatchObject({ credentials: 'same-origin' })
    expect(fetch.mock.calls[0][1].cache).toBeUndefined()
  })

  it('returns a backup as a Blob while decoding JSON failures before any download', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(
        new Response('SQLite format 3\\0', {
          headers: { 'Content-Type': 'application/octet-stream' },
        }),
      ),
    )
    const blob = await apiFetch<Blob>('/panel/api/v1/backups/download', { method: 'POST' })
    expect(blob).toBeInstanceOf(Blob)
    expect(await blob.text()).toContain('SQLite format 3')
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          Response.json(
            { code: 'reauthentication_required', message: 'Sign in again.' },
            { status: 403 },
          ),
        ),
    )
    await expect(
      apiFetch('/panel/api/v1/backups/download', { method: 'POST' }),
    ).rejects.toMatchObject({ status: 403, data: { code: 'reauthentication_required' } })
  })
  it('preserves the generated body, required request ID, cancellation and same-origin credentials', async () => {
    const fetch = vi.fn().mockResolvedValue(Response.json(result))
    vi.stubGlobal('fetch', fetch)
    setCsrfToken('session-csrf')
    const signal = new AbortController().signal
    const request = { user_id: 7, amount: 100 }
    const headers = { 'Idempotency-Key': result.request_id }
    await expect(giveBp(request, headers, { signal })).resolves.toEqual(result)
    const [url, options] = fetch.mock.calls[0] as [string, RequestInit]
    expect(url).toBe('/panel/api/v1/give-bp')
    expect(options).toMatchObject({
      method: 'POST',
      credentials: 'same-origin',
      signal,
      body: JSON.stringify(request),
    })
    expect(new Headers(options.headers).get('Idempotency-Key')).toBe(result.request_id)
    expect(new Headers(options.headers).get('X-CSRF-Token')).toBe('session-csrf')
    await giveBp(request, headers)
    expect(new Headers(fetch.mock.calls[1][1].headers).get('Idempotency-Key')).toBe(
      result.request_id,
    )
  })

  it('decodes declared errors with their stable codes and never retries a mutation itself', async () => {
    const error = ApiError.parse({
      code: 'request_conflict',
      message: 'Already used for another change.',
    })
    const fetch = vi.fn().mockResolvedValue(Response.json(error, { status: 409 }))
    vi.stubGlobal('fetch', fetch)
    await expect(
      giveBp({ user_id: 7, amount: 100 }, { 'Idempotency-Key': result.request_id }),
    ).rejects.toMatchObject({ status: 409, data: error })
    expect(fetch).toHaveBeenCalledTimes(1)
  })

  it('turns an unexpected proxy error into a usable typed error', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue(new Response('<html>Bad gateway</html>', { status: 502 })),
    )
    try {
      await getStaffSession()
      expect.unreachable()
    } catch (error) {
      expect(error).toBeInstanceOf(ApiHttpError)
      expect(ApiError.safeParse((error as ApiHttpError).data).success).toBe(true)
    }
  })

  it('validates test fixtures against Rust-derived schemas, including RFC 3339 timestamps', () => {
    const session = {
      user_id: 1,
      user: 'TestAdmin',
      role: 'admin',
      csrf_token: 'token',
      expires_at: '2026-11-16T12:00:00Z',
      mfa_enabled: true,
      mfa_required: true,
      limited: false,
      mfa_recent_until: null,
      recovery_codes_remaining: 10,
    }
    expect(StaffSession.safeParse(session).success).toBe(true)
    expect(StaffSession.safeParse({ ...session, expires_at: 'yesterday' }).success).toBe(false)
    expect(GiveBpResult.safeParse({ ...result, bp: '5100' }).success).toBe(false)
  })

  it('does not send the CSRF token to another origin', async () => {
    const fetch = vi.fn()
    vi.stubGlobal('fetch', fetch)
    setCsrfToken('secret')
    await expect(apiFetch('https://elsewhere.example/api', { method: 'POST' })).rejects.toThrow()
    expect(fetch).not.toHaveBeenCalled()
  })
})
