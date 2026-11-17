/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { GiftOutcome } from './giftOutcome';

export interface GiftResult {
  gift: GiftOutcome;
  message: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
