/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type SettingsRequest = {
  kind: 'set';
  name: string;
  value: string;
} | {
  kind: 'reset';
  name: string;
} | {
  kind: 'schedule_maintenance';
  /** @nullable */
  message?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  minutes: number;
} | {
  kind: 'cancel_maintenance';
};
