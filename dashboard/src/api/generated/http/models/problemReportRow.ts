/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * A report in a list: everything but its logs. (Own schema names: `Report` is the in-game player report.)
 */
export interface ProblemReportRow {
  /** The account's User ID, or its players' names for a key account. */
  account: string;
  /** When it came, in Unix seconds. */
  at: number;
  /**
     * The size of its logs in bytes.
     * @minimum 0
     */
  bytes: number;
  /** R-XXXX-XXXX */
  id: string;
  /** network, relay, server-lost, game-error or report. */
  kind: string;
  /** The launcher's version. */
  launcher: string;
  /** One line on what happened, from the launcher. */
  summary: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
