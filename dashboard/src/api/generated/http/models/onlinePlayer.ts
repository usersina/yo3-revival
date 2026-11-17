/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface OnlinePlayer {
  duelling: boolean;
  name: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  user_id?: number | null;
}
