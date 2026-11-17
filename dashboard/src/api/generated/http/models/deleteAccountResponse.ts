/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { DeleteAccountResult } from './deleteAccountResult';
import type { EffectState } from './effectState';

export interface DeleteAccountResponse {
  disconnect?: EffectState | null;
  result: DeleteAccountResult;
}
