/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface LinkedAccount {
  /** @nullable */
  login?: string | null;
  players: string[];
  /** What they share: `device`, `address` or both. */
  shared: string[];
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
