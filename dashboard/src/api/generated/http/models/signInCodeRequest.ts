/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface SignInCodeRequest {
  /**
     * @minLength 64
     * @maxLength 64
     */
  challenge: string;
  /**
     * An authenticator code (six digits) or an unused recovery code.
     * @minLength 6
     * @maxLength 32
     */
  code: string;
}
