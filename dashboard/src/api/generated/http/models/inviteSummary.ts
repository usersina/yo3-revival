/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { InviteClaim } from './inviteClaim';
import type { InviteState } from './inviteState';

export interface InviteSummary {
  claims: InviteClaim[];
  code: string;
  /** @nullable */
  expires_at?: string | null;
  issued_by: string;
  state: InviteState;
  /**
     * Remaining sign-ups, not the original limit. Null means unlimited.
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  uses?: number | null;
}
