/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { DuelRecord } from './duelRecord';

export interface PlayerDetail {
  /**
     * Additional card copies, excluding the starter deck. u16 card rows * u32 counts fit below 2^48, exactly in JS.
     * @minimum 0
     * @maximum 9007199254740991
     */
  additional_cards: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  card_kinds: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  level: number;
  name: string;
  /**
     * Live presence is unknown without a running hub. Never infer it from saved accounts.
     * @nullable
     */
  online?: boolean | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  owned_items: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  purchased_decks: number;
  record?: DuelRecord | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  slot: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  title_id: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  uploaded_decks: number;
}
