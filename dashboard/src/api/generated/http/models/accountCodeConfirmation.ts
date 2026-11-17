/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AccountCodeKind } from './accountCodeKind';

export interface AccountCodeConfirmation {
  expires_at: string;
  kind: AccountCodeKind;
  request_id: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
