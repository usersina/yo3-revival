/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { StaffSession } from './staffSession';

/**
 * The result of the password step: a code is still owed (`challenge`), or a session was opened (see its `limited`).
 */
export interface SignInOutcome {
  /**
     * Present with `code_required`: send it back with the code to `/sign-in/code` within five minutes.
     * @nullable
     */
  challenge?: string | null;
  code_required: boolean;
  session?: StaffSession | null;
}
