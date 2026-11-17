/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type GiftRequest = {
  /**
     * @minimum 1
     * @maximum 255
     */
  count: number;
  kind: 'cards';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /** @minimum 0 */
  row: number;
} | {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  deck_id: number;
  include_cards: boolean;
  kind: 'recipe';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
} | {
  kind: 'title';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /** @minimum 0 */
  title_id: number;
};
