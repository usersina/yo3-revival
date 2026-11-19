/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface CodeRequest {
  /**
     * An authenticator code (six digits) or an unused recovery code.
     * @minLength 6
     * @maxLength 32
     */
  code: string;
}
