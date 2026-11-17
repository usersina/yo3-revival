/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { Role } from './role';

export type AccountEditRequest = {
  kind: 'set_role';
  role: Role;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'add_player';
  /**
     * @minLength 1
     * @maxLength 16
     */
  name: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  kind: 'rename_player';
  /**
     * @minLength 1
     * @maxLength 16
     */
  name: string;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  player_id: number;
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
} | {
  /**
     * Null or omitted: derive from the game's language. Other codes must occur in the client's table.
     * @minLength 2
     * @maxLength 2
     * @nullable
     * @pattern ^[A-Za-z]{2}$
     */
  country?: string | null;
  kind: 'set_country';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
};
