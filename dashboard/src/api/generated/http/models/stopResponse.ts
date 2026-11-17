/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { EffectState } from './effectState';
import type { StopResult } from './stopResult';

export interface StopResponse {
  result: StopResult;
  stop: EffectState;
}
