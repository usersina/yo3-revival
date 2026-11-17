/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AccountKey } from './accountKey';
import type { KeyPlayer } from './keyPlayer';

export interface KeyAccountResult {
  key: AccountKey;
  players: KeyPlayer[];
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
