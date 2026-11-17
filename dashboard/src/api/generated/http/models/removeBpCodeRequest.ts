/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface RemoveBpCodeRequest {
  /**
     * @minLength 16
     * @maxLength 16
     * @pattern ^[A-Za-z0-9]{16}$
     */
  code: string;
}
