/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type GiftOutcome = {
  /**
     * @minimum 0
     * @maximum 255
     */
  added: number;
  kind: 'cards';
  /** @minimum 0 */
  row: number;
  /**
     * @minimum 0
     * @maximum 255
     */
  total: number;
} | {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  deck_id: number;
  include_cards: boolean;
  kind: 'recipe';
} | {
  already_owned: boolean;
  kind: 'title';
  /** @minimum 0 */
  title_id: number;
};
