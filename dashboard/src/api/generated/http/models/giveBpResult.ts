/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface GiveBpResult {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  bp: number;
  message: string;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
