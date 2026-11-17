/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ManageAccountKeyHeaders = {
  /**
   * Retain this ID and input on retry. A key is returned only after its audit is committed; never stored in a receipt.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
