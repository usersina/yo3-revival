/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * One entrant, in the order they entered.
 */
export interface EntrantView {
  name: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
}
