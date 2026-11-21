/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ChampionshipPrize } from './championshipPrize';
import type { ChampionshipState } from './championshipState';
import type { EntrantView } from './entrantView';

export interface ChampionshipView {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  championship_id: number;
  entrants: EntrantView[];
  entry_close: string;
  entry_open: string;
  /**
     * The lobby's `event_id`, which the game's requests carry.
     * @minimum 0
     * @maximum 4294967295
     */
  event_id: number;
  /**
     * Duels a player must have played to enter.
     * @minimum 0
     * @maximum 4294967295
     */
  min_duels: number;
  name: string;
  prize: ChampionshipPrize;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  season: number;
  second?: EntrantView | null;
  start: string;
  state: ChampionshipState;
  winner?: EntrantView | null;
  /**
     * The banner's year (Japan time) and the Championship's number in it.
     * @minimum 0
     * @maximum 4294967295
     */
  year: number;
}
