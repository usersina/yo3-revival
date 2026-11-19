/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ReportView } from './reportView';

export interface ReportList {
  checked_at: string;
  /**
     * Open reports in all, whatever the list shows.
     * @minimum 0
     * @maximum 4294967295
     */
  open: number;
  reports: ReportView[];
}
