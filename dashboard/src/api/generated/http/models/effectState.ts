/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type EffectState = typeof EffectState[keyof typeof EffectState];


export const EffectState = {
  pending: 'pending',
  completed: 'completed',
  interrupted: 'interrupted',
  failed: 'failed',
} as const;
