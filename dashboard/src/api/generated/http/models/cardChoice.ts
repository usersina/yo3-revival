/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface CardChoice {
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  art: number;
  description: string;
  /** @nullable */
  image_url?: string | null;
  /** @minimum 0 */
  konami_id: number;
  name: string;
  /** @minimum 0 */
  row: number;
  /** @nullable */
  thumbnail_url?: string | null;
}
