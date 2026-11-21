/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { ChampionshipPrize } from './championshipPrize';

/**
 * What to change in a Championship still waiting for its start; what is left out stays.
 */
export interface EditChampionshipRequest {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  championship_id: number;
  /** @nullable */
  entry_close?: string | null;
  /** @nullable */
  entry_open?: string | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  min_duels?: number | null;
  /**
     * @minLength 1
     * @maxLength 60
     * @nullable
     */
  name?: string | null;
  prize?: ChampionshipPrize | null;
  /** @nullable */
  start?: string | null;
}
