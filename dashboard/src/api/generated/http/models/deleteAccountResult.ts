/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface DeleteAccountResult {
  message: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  players: number;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
