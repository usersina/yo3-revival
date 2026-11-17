/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { EffectState } from './effectState';
import type { ModerationResult } from './moderationResult';

export interface ModerationResponse {
  disconnect?: EffectState | null;
  result: ModerationResult;
}
