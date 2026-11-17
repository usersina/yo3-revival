/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type AccountCodeState = typeof AccountCodeState[keyof typeof AccountCodeState];


export const AccountCodeState = {
  ready: 'ready',
  expired: 'expired',
  used_or_replaced: 'used_or_replaced',
} as const;
