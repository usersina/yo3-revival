/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type HandleReportHeaders = {
  /**
   * One ID per change; marking the same report the same way again answers as done.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
