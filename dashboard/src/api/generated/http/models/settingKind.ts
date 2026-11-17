/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { RegulationChoice } from './regulationChoice';

export type SettingKind = {
  kind: 'text';
  /** @minimum 0 */
  max_length: number;
} | {
  kind: 'number';
  /** @minimum 0 */
  max: number;
  /** @minimum 0 */
  min: number;
} | {
  choices: string[];
  kind: 'choice';
} | {
  kind: 'version';
} | {
  choices: RegulationChoice[];
  kind: 'regulation';
} | {
  kind: 'news';
};
