/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ListProblemReportsParams = {
  /**
   * A User ID or a player's name: only that account's reports.
   * @maxLength 64
   */
  who?: string;
  /**
   * @minimum 1
   * @maximum 500
   */
  limit?: number;
};
