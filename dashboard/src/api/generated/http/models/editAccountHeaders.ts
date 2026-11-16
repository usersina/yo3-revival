/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type EditAccountHeaders = {
  /**
   * One ID per change. Retain it on an uncertain outcome; retries never reapply a committed edit.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
