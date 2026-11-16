/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface CreateInviteRequest {
  /**
     * Days from the first successful creation. Omit for no expiry; retries keep the original expiry.
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  days?: number | null;
  /**
     * Number of sign-ups. Omit for unlimited.
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  uses?: number | null;
}
