/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ChangeSettingsHeaders = {
  /**
   * Retain this ID and input on retry. Retries return the original result without restoring old settings or moving a countdown.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
