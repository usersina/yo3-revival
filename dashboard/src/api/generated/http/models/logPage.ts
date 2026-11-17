/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { LogEntry } from './logEntry';

export interface LogPage {
  checked_at: string;
  lines: LogEntry[];
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  retention_lines: number;
}
