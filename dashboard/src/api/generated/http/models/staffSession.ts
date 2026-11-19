/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Role } from './role';

export interface StaffSession {
  csrf_token: string;
  /** Idle expiry, refreshed by an authenticated request; capped by the absolute session lifetime. */
  expires_at: string;
  /** An enrollment-only session: every route but enrollment, sign-out and this session answers `mfa_enrollment_required`. */
  limited: boolean;
  /** The account has an active authenticator (TOTP). */
  mfa_enabled: boolean;
  /**
     * When the authenticator's recent check ends; none: sensitive operations ask for a code first.
     * @nullable
     */
  mfa_recent_until?: string | null;
  /** The server requires an authenticator of every staff account (the default policy). */
  mfa_required: boolean;
  /**
     * Unused recovery codes left.
     * @minimum 0
     * @maximum 100
     */
  recovery_codes_remaining: number;
  role: Role;
  user: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
