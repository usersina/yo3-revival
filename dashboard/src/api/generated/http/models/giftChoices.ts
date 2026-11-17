/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { CardChoice } from './cardChoice';
import type { RecipeChoice } from './recipeChoice';
import type { TitleChoice } from './titleChoice';

export interface GiftChoices {
  cards: CardChoice[];
  more_cards: boolean;
  more_recipes: boolean;
  more_titles: boolean;
  recipes: RecipeChoice[];
  titles: TitleChoice[];
}
