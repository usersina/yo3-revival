/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface DuelRecord {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  draws: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  losses: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  wins: number;
}
