/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface RecipeChoice {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  card_count: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  deck_id: number;
  name: string;
}
