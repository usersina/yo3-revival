/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { EventView } from './eventView';

export interface EventList {
  checked_at: string;
  events: EventView[];
}
