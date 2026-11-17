/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { SettingKind } from './settingKind';

export interface SettingRow {
  applies: string;
  default_value: string;
  help: string;
  input: SettingKind;
  name: string;
  overridden: boolean;
  value: string;
}
