/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * Where an event is in time, for the staff (the game sees only its `status` number).
 */
export type Phase = typeof Phase[keyof typeof Phase];


export const Phase = {
  scheduled: 'scheduled',
  running: 'running',
  finished: 'finished',
  cancelled: 'cancelled',
} as const;
