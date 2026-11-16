import type { ApiError } from './generated/http/models'
import { ApiError as ApiErrorSchema } from './generated/validation/schemas'

let csrfToken: string | undefined

// Kept only in memory; the HttpOnly session cookie is never read by JavaScript.
export function setCsrfToken(value: string | undefined) {
  csrfToken = value
}

export class ApiHttpError<T = ApiError> extends Error {
  constructor(
    readonly status: number,
    readonly data: T,
    message: string,
  ) {
    super(message)
    this.name = 'ApiHttpError'
  }
}

// Orval uses this alias for the generated hooks' error type.
export type ErrorType<T> = ApiHttpError<T>

export async function apiFetch<T>(url: string, options?: RequestInit): Promise<T> {
  const target = new URL(url, window.location.origin)
  if (target.origin !== window.location.origin || !target.pathname.startsWith('/panel/api/')) {
    throw new Error('Refusing an API request outside this dashboard origin.')
  }
  const headers = new Headers(options?.headers)
  if (csrfToken && options?.method && !['GET', 'HEAD'].includes(options.method.toUpperCase())) {
    headers.set('X-CSRF-Token', csrfToken)
  }
  const response = await fetch(url, { ...options, headers, credentials: 'same-origin' })
  const data: unknown = await response.json().catch(() => null)
  if (!response.ok) {
    const decoded = ApiErrorSchema.safeParse(data)
    const error: ApiError = decoded.success
      ? decoded.data
      : {
          code: 'internal_error',
          message: `The server returned HTTP ${response.status}. Try again shortly.`,
        }
    if (response.status === 401) setCsrfToken(undefined)
    throw new ApiHttpError(response.status, error, error.message)
  }
  return data as T
}
