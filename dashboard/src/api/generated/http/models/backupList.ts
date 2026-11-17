/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { BackupEntry } from './backupEntry';

export interface BackupList {
  backups: BackupEntry[];
  checked_at: string;
  downloads_allowed: boolean;
}
