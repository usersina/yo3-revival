/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface AccountEditResult {
  message: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  player_id?: number | null;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
