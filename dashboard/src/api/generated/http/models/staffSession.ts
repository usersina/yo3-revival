/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Role } from './role';

export interface StaffSession {
  csrf_token: string;
  /** Idle expiry, refreshed by an authenticated request; capped by the absolute session lifetime. */
  expires_at: string;
  role: Role;
  user: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
