/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type SetAccountBanlistHeaders = {
  /**
   * One ID per change; the change is the same when repeated.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
