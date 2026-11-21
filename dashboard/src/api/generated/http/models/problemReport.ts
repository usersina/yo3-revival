/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ProblemReportRow } from './problemReportRow';

/**
 * One report with the server's lines and the launcher's log.
 */
export type ProblemReport = ProblemReportRow & {
  /** The launcher's session log, credentials scrubbed. */
  logs: string;
  /** The server's own lines about this account from the minutes before. */
  server_log: string[];
  /** The whole report as the text file staff download. */
  text: string;
};
