/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type GivePlayerGiftHeaders = {
  /**
   * Retain this ID and input on retry. Retries never give duplicate cards or repeat mail or game notifications.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
