/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { BackupHealth } from './backupHealth';
import type { OnlinePlayer } from './onlinePlayer';

export interface ServerOverview {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  accounts: number;
  backup: BackupHealth;
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
  engines: number;
  /**
     * Decimal bytes, preserving precision in JavaScript. Missing on unsupported operating systems.
     * @nullable
     */
  memory_bytes?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  online: number;
  online_players: OnlinePlayer[];
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
