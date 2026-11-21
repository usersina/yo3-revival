/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ChampionshipPrize } from './championshipPrize';

export interface CreateChampionshipRequest {
  entry_close: string;
  /**
     * When entries open (omitted: now); they close at `entry_close` and the draw is at `start`, all on one day.
     * @nullable
     */
  entry_open?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  min_duels?: number;
  /**
     * @minLength 1
     * @maxLength 60
     */
  name: string;
  prize?: ChampionshipPrize;
  start: string;
}
