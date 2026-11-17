/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface SettingsResult {
  /**
     * Committed deadline, not a live status. A retry returns this original deadline even after later changes.
     * @nullable
     */
  maintenance_at?: string | null;
  message: string;
  request_id: string;
}
