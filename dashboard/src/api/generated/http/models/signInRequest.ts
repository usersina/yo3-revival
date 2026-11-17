/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface SignInRequest {
  password: string;
  /**
     * @minLength 1
     * @maxLength 64
     */
  user: string;
}
