/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface DeleteAccountRequest {
  /**
     * Must match user_id. The dashboard asks the administrator to type this account number.
     * @minimum 0
     * @maximum 4294967295
     */
  confirm_user_id: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
}
