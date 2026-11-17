/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface ServerOverview {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  accounts: number;
  can_stop: boolean;
  checked_at: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  duels: number;
  /** @nullable */
  engine_problem?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  online: number;
  persistence_healthy: boolean;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  players: number;
  run_id: string;
  /**
     * @minimum 0
     * @nullable
     */
  uptime_seconds?: number | null;
  version: string;
}
