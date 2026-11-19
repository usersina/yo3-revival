/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { BanlistChoice } from './banlistChoice';

export interface BanlistChoices {
  checked_at: string;
  choices: BanlistChoice[];
  /** What an account without a choice logs in with. */
  server_default: BanlistChoice;
}
