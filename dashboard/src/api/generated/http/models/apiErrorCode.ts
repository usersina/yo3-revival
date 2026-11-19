/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ApiErrorCode = typeof ApiErrorCode[keyof typeof ApiErrorCode];


export const ApiErrorCode = {
  invalid_request: 'invalid_request',
  unauthenticated: 'unauthenticated',
  forbidden: 'forbidden',
  invalid_origin: 'invalid_origin',
  csrf_invalid: 'csrf_invalid',
  rate_limited: 'rate_limited',
  not_found: 'not_found',
  already_exists: 'already_exists',
  request_conflict: 'request_conflict',
  persistence_failed: 'persistence_failed',
  reauthentication_required: 'reauthentication_required',
  invalid_code: 'invalid_code',
  mfa_enrollment_required: 'mfa_enrollment_required',
  method_not_allowed: 'method_not_allowed',
  payload_too_large: 'payload_too_large',
  unsupported_media_type: 'unsupported_media_type',
  internal_error: 'internal_error',
} as const;
