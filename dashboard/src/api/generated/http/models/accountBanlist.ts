/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { BanlistChoice } from './banlistChoice';

/**
 * The banlist an account's game logs in with.
 */
export type AccountBanlist = BanlistChoice & {
  /** True when the account (or the staff) chose it; false when it is the server's `login-regulation`. */
  chosen: boolean;
};
