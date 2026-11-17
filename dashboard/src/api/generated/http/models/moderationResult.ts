/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface ModerationResult {
  message: string;
  operation: string;
  request_id: string;
  /** @nullable */
  until?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
