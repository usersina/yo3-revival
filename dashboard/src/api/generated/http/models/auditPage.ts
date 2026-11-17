/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AuditEntry } from './auditEntry';

export interface AuditPage {
  checked_at: string;
  entries: AuditEntry[];
  /** @nullable */
  next_before?: string | null;
}
