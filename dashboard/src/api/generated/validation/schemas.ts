/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import * as zod from 'zod';

export const playerSummaryPlayerIdMin = 0;
export const playerSummaryPlayerIdMax = 4294967295;

export const playerSummarySlotMin = 0;
export const playerSummarySlotMax = 4294967295;



export const PlayerSummary = zod.object({
  "name": zod.string(),
  "player_id": zod.int().min(playerSummaryPlayerIdMin).max(playerSummaryPlayerIdMax),
  "slot": zod.int().min(playerSummarySlotMin).max(playerSummarySlotMax)
});

export type PlayerSummary = zod.input<typeof PlayerSummary>;
export type PlayerSummaryOutput = zod.output<typeof PlayerSummary>;

export const Role = zod.enum(['player', 'moderator', 'admin']);

export type Role = zod.input<typeof Role>;
export type RoleOutput = zod.output<typeof Role>;

export const accountSummaryBpMin = 0;
export const accountSummaryBpMax = 4294967295;

export const accountSummaryUserIdMin = 0;
export const accountSummaryUserIdMax = 4294967295;



export const AccountSummary = zod.object({
  "bp": zod.int().min(accountSummaryBpMin).max(accountSummaryBpMax),
  "login": zod.string().nullish(),
  "players": zod.array(PlayerSummary),
  "role": Role,
  "user_id": zod.int().min(accountSummaryUserIdMin).max(accountSummaryUserIdMax)
}).describe('A projection of existing account state: no keys, password hashes, device tokens or invented dates/presence.');

export type AccountSummary = zod.input<typeof AccountSummary>;
export type AccountSummaryOutput = zod.output<typeof AccountSummary>;

export const AccountList = zod.object({
  "accounts": zod.array(AccountSummary)
});

export type AccountList = zod.input<typeof AccountList>;
export type AccountListOutput = zod.output<typeof AccountList>;

export const ApiErrorCode = zod.enum(['invalid_request', 'unauthenticated', 'forbidden', 'invalid_origin', 'csrf_invalid', 'rate_limited', 'not_found', 'request_conflict', 'persistence_failed', 'method_not_allowed', 'payload_too_large', 'unsupported_media_type', 'internal_error']);

export type ApiErrorCode = zod.input<typeof ApiErrorCode>;
export type ApiErrorCodeOutput = zod.output<typeof ApiErrorCode>;

export const ApiError = zod.object({
  "code": ApiErrorCode,
  "message": zod.string()
});

export type ApiError = zod.input<typeof ApiError>;
export type ApiErrorOutput = zod.output<typeof ApiError>;

export const giveBpByIdRequestAmountMax = 4294967295;

export const giveBpByIdRequestUserIdMin = 0;
export const giveBpByIdRequestUserIdMax = 4294967295;



export const GiveBpByIdRequest = zod.object({
  "amount": zod.int().min(1).max(giveBpByIdRequestAmountMax),
  "user_id": zod.int().min(giveBpByIdRequestUserIdMin).max(giveBpByIdRequestUserIdMax)
}).describe('The dashboard always targets the stable account number, never a renameable player or login label.');

export type GiveBpByIdRequest = zod.input<typeof GiveBpByIdRequest>;
export type GiveBpByIdRequestOutput = zod.output<typeof GiveBpByIdRequest>;

export const giveBpResultBpMin = 0;
export const giveBpResultBpMax = 4294967295;

export const giveBpResultUserIdMin = 0;
export const giveBpResultUserIdMax = 4294967295;



export const GiveBpResult = zod.object({
  "bp": zod.int().min(giveBpResultBpMin).max(giveBpResultBpMax),
  "message": zod.string(),
  "request_id": zod.string(),
  "user_id": zod.int().min(giveBpResultUserIdMin).max(giveBpResultUserIdMax)
});

export type GiveBpResult = zod.input<typeof GiveBpResult>;
export type GiveBpResultOutput = zod.output<typeof GiveBpResult>;

export const signInRequestUserMax = 64;



export const SignInRequest = zod.object({
  "password": zod.string(),
  "user": zod.string().min(1).max(signInRequestUserMax)
});

export type SignInRequest = zod.input<typeof SignInRequest>;
export type SignInRequestOutput = zod.output<typeof SignInRequest>;

export const SignedOut = zod.object({
  "signed_out": zod.boolean()
});

export type SignedOut = zod.input<typeof SignedOut>;
export type SignedOutOutput = zod.output<typeof SignedOut>;

export const staffSessionUserIdMin = 0;
export const staffSessionUserIdMax = 4294967295;



export const StaffSession = zod.object({
  "csrf_token": zod.string(),
  "expires_at": zod.iso.datetime({"offset":true}).describe('Idle expiry, refreshed by an authenticated request; capped by the absolute session lifetime.'),
  "role": Role,
  "user": zod.string(),
  "user_id": zod.int().min(staffSessionUserIdMin).max(staffSessionUserIdMax)
});

export type StaffSession = zod.input<typeof StaffSession>;
export type StaffSessionOutput = zod.output<typeof StaffSession>;
export const ListAccounts200Response = AccountList

export const ListAccounts400Response = ApiError

export const ListAccounts401Response = ApiError

export const ListAccounts403Response = ApiError

export const ListAccounts405Response = ApiError

export const ListAccounts413Response = ApiError

export const ListAccounts415Response = ApiError

export const ListAccounts500Response = ApiError


export const giveBpHeaderIdempotencyKeyMin = 16;
export const giveBpHeaderIdempotencyKeyMax = 128;


export const giveBpHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const GiveBpHeader = zod.object({
  "Idempotency-Key": zod.string().min(giveBpHeaderIdempotencyKeyMin).max(giveBpHeaderIdempotencyKeyMax).regex(giveBpHeaderIdempotencyKeyRegExp).describe('Create once per intended grant; reuse on every retry, including after a lost response.')
})

export const GiveBpBody = GiveBpByIdRequest

export const GiveBp200Response = GiveBpResult

export const GiveBp400Response = ApiError

export const GiveBp401Response = ApiError

export const GiveBp403Response = ApiError

export const GiveBp404Response = ApiError

export const GiveBp405Response = ApiError

export const GiveBp409Response = ApiError

export const GiveBp413Response = ApiError

export const GiveBp415Response = ApiError

export const GiveBp500Response = ApiError

export const GiveBp503Response = ApiError


export const GetStaffSession200Response = StaffSession

export const GetStaffSession400Response = ApiError

export const GetStaffSession401Response = ApiError

export const GetStaffSession403Response = ApiError

export const GetStaffSession405Response = ApiError

export const GetStaffSession413Response = ApiError

export const GetStaffSession415Response = ApiError

export const GetStaffSession500Response = ApiError


export const signInStaffHeaderIdempotencyKeyMin = 16;
export const signInStaffHeaderIdempotencyKeyMax = 128;


export const signInStaffHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const SignInStaffHeader = zod.object({
  "Idempotency-Key": zod.string().min(signInStaffHeaderIdempotencyKeyMin).max(signInStaffHeaderIdempotencyKeyMax).regex(signInStaffHeaderIdempotencyKeyRegExp).describe('One ID per sign-in attempt. Authentication does not create a durable operation receipt.')
})

export const SignInStaffBody = SignInRequest

export const SignInStaff200Response = StaffSession

export const SignInStaff400Response = ApiError

export const SignInStaff401Response = ApiError

export const SignInStaff403Response = ApiError

export const SignInStaff405Response = ApiError

export const SignInStaff413Response = ApiError

export const SignInStaff415Response = ApiError

export const SignInStaff429Response = ApiError

export const SignInStaff500Response = ApiError


export const signOutStaffHeaderIdempotencyKeyMin = 16;
export const signOutStaffHeaderIdempotencyKeyMax = 128;


export const signOutStaffHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const SignOutStaffHeader = zod.object({
  "Idempotency-Key": zod.string().min(signOutStaffHeaderIdempotencyKeyMin).max(signOutStaffHeaderIdempotencyKeyMax).regex(signOutStaffHeaderIdempotencyKeyRegExp)
})

export const SignOutStaff200Response = SignedOut

export const SignOutStaff400Response = ApiError

export const SignOutStaff401Response = ApiError

export const SignOutStaff403Response = ApiError

export const SignOutStaff405Response = ApiError

export const SignOutStaff413Response = ApiError

export const SignOutStaff415Response = ApiError

export const SignOutStaff500Response = ApiError
