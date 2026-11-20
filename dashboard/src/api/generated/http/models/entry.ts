/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * A player's standing in one event.
 */
export interface Entry {
  /**
     * The best score reached: what the prize and the Tournament Points count (a streak ends, the best stays).
     * @minimum 0
     */
  best: number;
  /** @minimum 0 */
  draws: number;
  /** @minimum 0 */
  losses: number;
  name: string;
  /** The prize has been paid (or there was none). */
  paid: boolean;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /**
     * What the client shows as `event_score`: the current win streak (Gunslinger), the wins or the stars.
     * @minimum 0
     */
  score: number;
  /** @minimum 0 */
  wins: number;
}
