/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ProblemReportRow } from './problemReportRow';

export interface ProblemReportList {
  checked_at: string;
  /** Newest first, without their logs. */
  reports: ProblemReportRow[];
}
