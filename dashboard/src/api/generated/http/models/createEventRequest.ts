/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Kind } from './kind';
import type { Prize } from './prize';

export interface CreateEventRequest {
  end: string;
  kind: Kind;
  /**
     * @minLength 1
     * @maxLength 60
     */
  name: string;
  prize?: Prize;
  /**
     * A banlist's id (`/panel/api/v1/banlists`); omitted: any.
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  regulation?: number | null;
  start: string;
}
