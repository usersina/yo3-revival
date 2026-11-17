/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AccountCodeConfirmation } from './accountCodeConfirmation';
import type { AccountCodeState } from './accountCodeState';

export interface AccountCodeResponse {
  /** Immutable issuance receipt. A retry never creates or extends a code. */
  confirmation: AccountCodeConfirmation;
  /** Live availability, checked now; not part of the stored receipt. */
  state: AccountCodeState;
}
