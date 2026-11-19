/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Status } from './status';

export interface Report {
  /** RFC 3339 UTC, to the second. */
  at: string;
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
  reporter: string;
  /** @minimum 0 */
  reporter_id: number;
  status: Status;
  target: string;
  /** @minimum 0 */
  target_id: number;
}
