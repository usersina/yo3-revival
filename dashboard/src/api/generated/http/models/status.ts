/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * Where a report stands.
 */
export type Status = typeof Status[keyof typeof Status];


export const Status = {
  open: 'open',
  handled: 'handled',
  dismissed: 'dismissed',
} as const;
