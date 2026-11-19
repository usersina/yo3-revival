/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface SetBanlistRequest {
  /**
     * A banlist's id from the choices; null or omitted goes back to the server's.
     * @minimum 0
     * @maximum 4294967295
     * @nullable
     */
  banlist?: number | null;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
