/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { BpCodeClaim } from './bpCodeClaim';

export interface BpCodeSummary {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  bp: number;
  claims: BpCodeClaim[];
  code: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  uses?: number | null;
}
