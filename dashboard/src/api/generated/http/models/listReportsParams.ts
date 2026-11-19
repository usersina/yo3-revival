/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Status } from './status';

export type ListReportsParams = {
  /** open, handled or dismissed; all when omitted. */
  status?: Status;
  /**
   * @minimum 1
   * @maximum 500
   */
  limit?: number;
};
