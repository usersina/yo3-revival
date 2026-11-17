/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface AuditEntry {
  actor: string;
  /** @nullable */
  after?: string | null;
  at: string;
  /**
     * Pretty-printed JSON for display as text, not executable HTML or a business-operation response.
     * @nullable
     */
  before?: string | null;
  /** @nullable */
  detail?: string | null;
  /** Decimal SQLite row ID. Never convert this to a JavaScript number. */
  id: string;
  operation: string;
  origin: string;
  target: string;
}
