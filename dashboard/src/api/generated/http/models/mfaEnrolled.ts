/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { StaffSession } from './staffSession';

/**
 * An authenticator was activated. The recovery codes are shown once and are kept only as digests.
 */
export interface MfaEnrolled {
  recovery_codes: string[];
  session: StaffSession;
}
