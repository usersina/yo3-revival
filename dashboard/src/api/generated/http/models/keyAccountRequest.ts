/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type KeyAccountRequest = {
  kind: 'create';
  /** @maxItems 4 */
  names: string[];
} | {
  kind: 'reveal';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
};
