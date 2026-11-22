/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Status } from './status';

export interface Report {
  /** RFC 3339 UTC, to the second. */
  at: string;
  /** A duel problem (`bug`) rather than a report about a player. */
  duel_problem?: boolean;
  /** @nullable */
  handled_at?: string | null;
  /** @nullable */
  handled_by?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  id: number;
  /** @nullable */
  note?: string | null;
  reason: string;
  /**
     * A duel problem's recording id (`bug` in the chat), pinned in the data folder's `recordings/pinned/`. Null for a
     * report about a player, and for a duel problem sent without a recent duel.
     * @nullable
     */
  recording?: string | null;
  reporter: string;
  /** @minimum 0 */
  reporter_id: number;
  status: Status;
  target: string;
  /** @minimum 0 */
  target_id: number;
}
