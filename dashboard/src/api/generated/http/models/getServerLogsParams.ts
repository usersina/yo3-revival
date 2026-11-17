/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type GetServerLogsParams = {
  /**
   * @minimum 1
   * @maximum 500
   */
  limit?: number;
  /** Only warnings and errors */
  problems?: boolean;
};
