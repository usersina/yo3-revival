/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import type { AccessCode } from './accessCode';

export type IssueAccountCodeRequest = {
  code: AccessCode;
  kind: 'create_account';
  /**
     * @minLength 3
     * @maxLength 16
     * @pattern ^[A-Za-z0-9]+$
     */
  user: string;
} | {
  code: AccessCode;
  kind: 'reset_password';
  /**
     * @minimum 0
     * @maximum 4294967295
     */
  user_id: number;
};
