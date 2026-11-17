/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface StopRequest {
  /**
     * @minLength 64
     * @maxLength 64
     * @pattern ^[a-f0-9]{64}$
     */
  run_id: string;
}
