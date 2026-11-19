/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */

/**
 * The seed of a new authenticator, shown to its owner once. The QR code is the `otpauth_uri`.
 */
export interface MfaEnrollmentStart {
  account: string;
  issuer: string;
  otpauth_uri: string;
  secret: string;
}
