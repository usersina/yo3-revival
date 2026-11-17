/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

export interface DownloadBackupRequest {
  /**
     * @minLength 1
     * @maxLength 96
     * @pattern ^[A-Za-z0-9][A-Za-z0-9._-]*$
     */
  name: string;
}
