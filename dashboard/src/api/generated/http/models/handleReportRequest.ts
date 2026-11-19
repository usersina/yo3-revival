/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Status } from './status';

export interface HandleReportRequest {
  /**
     * What was done, shown with the report (optional).
     * @maxLength 300
     */
  note?: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  report_id: number;
  /** handled or dismissed. */
  status: Status;
}
