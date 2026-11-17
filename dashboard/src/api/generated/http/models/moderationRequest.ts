/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export type ModerationRequest = {
  kind: 'kick';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  /**
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  days?: number | null;
  kind: 'ban';
  /** @maxLength 200 */
  reason: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  /**
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  days?: number | null;
  kind: 'restrict';
  /** @maxLength 200 */
  reason: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'lift';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'mute';
  /**
     * @minimum 1
     * @maximum 4294967295
     * @nullable
     */
  minutes?: number | null;
  /** @maxLength 200 */
  reason: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'unmute';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'sign_out';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
};
