/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Entry } from './entry';
import type { Kind } from './kind';
import type { Phase } from './phase';
import type { Prize } from './prize';

export interface EventView {
  /** What the kind's score counts: win streak, wins or stars. */
  counts: string;
  end: string;
  /**
     * The players who entered its lobby.
     * @minimum 0
     * @maximum 4294967295
     */
  entrants: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  event_id: number;
  kind: Kind;
  name: string;
  phase: Phase;
  prize: Prize;
  /**
     * The banlist entrants must log in with (a regulation id); none: any.
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  regulation?: number | null;
  /** The ten best standings. */
  standings: Entry[];
  start: string;
}
