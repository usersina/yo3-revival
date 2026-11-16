/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface CreateInviteResult {
  code: string;
  /** @nullable */
  expires_at?: string | null;
  message: string;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  uses?: number | null;
}
