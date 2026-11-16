/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AccountMute } from './accountMute';
import type { AccountStanding } from './accountStanding';
import type { PlayerDetail } from './playerDetail';
import type { Role } from './role';

export interface AccountDetail {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  bp: number;
  checked_at: string;
  /** @nullable */
  country?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  device_count: number;
  has_key: boolean;
  /** @nullable */
  login?: string | null;
  muted?: AccountMute | null;
  players: PlayerDetail[];
  reset_pending: boolean;
  role: Role;
  standing?: AccountStanding | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
