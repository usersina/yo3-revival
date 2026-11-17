/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type AccountStanding = {
  kind: 'active';
} | {
  kind: 'restricted';
  reason: string;
  /** @nullable */
  until?: string | null;
} | {
  kind: 'banned';
  reason: string;
  /** @nullable */
  until?: string | null;
};
