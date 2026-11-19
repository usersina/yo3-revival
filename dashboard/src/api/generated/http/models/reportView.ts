/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Report } from './report';

/**
 * A report with the accounts of the two players, for the page's links.
 */
export type ReportView = Report & ({
  /**
     * The account of the reporting player (none once deleted).
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  reporter_user_id?: number | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  target_user_id?: number | null;
});
