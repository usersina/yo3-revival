/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type BackupHealth = {
  kind: 'none';
} | {
  at: string;
  kind: 'available';
  name: string;
} | {
  kind: 'unavailable';
  message: string;
};
