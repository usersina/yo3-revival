/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface BpCodeClaim {
  /** @nullable */
  at?: string | null;
  players: string[];
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
