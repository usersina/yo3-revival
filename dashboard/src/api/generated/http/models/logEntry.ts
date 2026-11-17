/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { LogLevel } from './logLevel';

export interface LogEntry {
  at: string;
  level: LogLevel;
  text: string;
}
