/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { SettingRow } from './settingRow';

export interface SettingsList {
  checked_at: string;
  maintenance_active: boolean;
  /** @nullable */
  maintenance_at?: string | null;
  settings: SettingRow[];
}
