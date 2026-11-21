/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * Where a Championship is: waiting for its start time (entries open from `entry_open`), being played, or over.
 */
export type ChampionshipState = typeof ChampionshipState[keyof typeof ChampionshipState];


export const ChampionshipState = {
  scheduled: 'scheduled',
  cancelled: 'cancelled',
  running: 'running',
  finished: 'finished',
} as const;
