/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * The dashboard always targets the stable account number, never a renameable player or login label.
 */
export interface GiveBpByIdRequest {
  /**
     * @minimum 1
     * @maximum 4294967295
     */
  amount: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
