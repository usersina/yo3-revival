/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type RemoveBpCodeHeaders = {
  /**
   * One ID per removal. Retain it on an uncertain outcome; do not generate a new one on retry.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
