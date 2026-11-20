/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * What an event pays, in BP (the original's Special Packs don't exist here): to everyone who played a duel of it, and
 * besides to everyone whose best score reached the threshold.
 */
export interface Prize {
  /**
     * @minimum 0
     * @maximum 1000000
     */
  participation_bp?: number;
  /**
     * The best score that earns `threshold_bp` (0: no threshold).
     * @minimum 0
     * @maximum 9999
     */
  threshold?: number;
  /**
     * @minimum 0
     * @maximum 1000000
     */
  threshold_bp?: number;
}
