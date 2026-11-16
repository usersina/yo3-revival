/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { PlayerSummary } from './playerSummary';
import type { Role } from './role';

/**
 * A projection of existing account state: no keys, password hashes, device tokens or invented dates/presence.
 */
export interface AccountSummary {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  bp: number;
  /** @nullable */
  login?: string | null;
  players: PlayerSummary[];
  role: Role;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
