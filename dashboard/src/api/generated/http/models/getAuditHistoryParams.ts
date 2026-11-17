/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type GetAuditHistoryParams = {
  /**
   * @minimum 1
   * @maximum 200
   */
  limit?: number;
  /**
   * Decimal ID returned by next_before; never a JS number
   * @minLength 1
   * @maxLength 19
   * @pattern ^[1-9][0-9]*$
   */
  before?: string;
  /** @maxLength 80 */
  q?: string;
};
