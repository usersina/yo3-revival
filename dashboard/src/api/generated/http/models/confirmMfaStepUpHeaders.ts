/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ConfirmMfaStepUpHeaders = {
  /**
   * One ID per attempt. Authentication does not create a durable operation receipt.
   * @minLength 16
   * @maxLength 128
   * @pattern ^[A-Za-z0-9_-]+$
   */
  'Idempotency-Key': string;
};
