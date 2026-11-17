/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { CreateBackupResult } from './createBackupResult';
import type { EffectState } from './effectState';

export interface CreateBackupResponse {
  available: boolean;
  backup: EffectState;
  result: CreateBackupResult;
}
