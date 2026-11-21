/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * What a Championship pays, in BP: to everyone drawn into the Final, to the finalists (both) and to the winner, which add up.
 */
export interface ChampionshipPrize {
  /**
     * @minimum 0
     * @maximum 1000000
     */
  finalist_bp?: number;
  /**
     * @minimum 0
     * @maximum 1000000
     */
  participation_bp?: number;
  /**
     * @minimum 0
     * @maximum 1000000
     */
  winner_bp?: number;
}
