/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface CreateBpCodeRequest {
  /**
     * @minimum 1
     * @maximum 4294967295
     */
  bp: number;
  /**
     * Omit for a generated code. Codes are case insensitive and stored in uppercase.
     * @minLength 16
     * @maxLength 16
     * @nullable
     * @pattern ^[A-Za-z0-9]{16}$
     */
  code?: string | null;
  /**
     * Number of accounts that can claim it. Omit for unlimited; each account can claim it once.
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  uses?: number | null;
}
