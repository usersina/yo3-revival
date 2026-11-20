/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Prize } from './prize';

/**
 * What to change in an event that hasn't finished; what is left out stays.
 */
export interface EditEventRequest {
  /** @nullable */
  end?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  event_id: number;
  /**
     * @minLength 1
     * @maxLength 60
     * @nullable
     */
  name?: string | null;
  prize?: Prize | null;
  /**
     * A banlist's id; 0 for any.
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  regulation?: number | null;
  /** @nullable */
  start?: string | null;
}
