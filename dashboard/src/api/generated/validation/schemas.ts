/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import * as zod from 'zod';

export const AccessCodeMin = 16;
export const AccessCodeMax = 19;


export const AccessCodeRegExp = new RegExp('^([A-Za-z0-9]{16}|[A-Za-z0-9]{4}(-[A-Za-z0-9]{4}){3})$');


export const AccessCode = zod.string().min(AccessCodeMin).max(AccessCodeMax).regex(AccessCodeRegExp);

export type AccessCode = zod.input<typeof AccessCode>;
export type AccessCodeOutput = zod.output<typeof AccessCode>;

export const AccountCodeKind = zod.enum(['setup', 'reset']);

export type AccountCodeKind = zod.input<typeof AccountCodeKind>;
export type AccountCodeKindOutput = zod.output<typeof AccountCodeKind>;

export const accountCodeConfirmationUserIdMin = 0;
export const accountCodeConfirmationUserIdMax = 4294967295;



export const AccountCodeConfirmation = zod.object({
  "expires_at": zod.iso.datetime({"offset":true}),
  "kind": AccountCodeKind,
  "request_id": zod.string(),
  "user_id": zod.int().min(accountCodeConfirmationUserIdMin).max(accountCodeConfirmationUserIdMax)
});

export type AccountCodeConfirmation = zod.input<typeof AccountCodeConfirmation>;
export type AccountCodeConfirmationOutput = zod.output<typeof AccountCodeConfirmation>;

export const AccountCodeState = zod.enum(['ready', 'expired', 'used_or_replaced']);

export type AccountCodeState = zod.input<typeof AccountCodeState>;
export type AccountCodeStateOutput = zod.output<typeof AccountCodeState>;

export const AccountCodeResponse = zod.object({
  "confirmation": AccountCodeConfirmation.describe('Immutable issuance receipt. A retry never creates or extends a code.'),
  "state": AccountCodeState.describe('Live availability, checked now; not part of the stored receipt.')
});

export type AccountCodeResponse = zod.input<typeof AccountCodeResponse>;
export type AccountCodeResponseOutput = zod.output<typeof AccountCodeResponse>;

export const AccountMute = zod.object({
  "reason": zod.string(),
  "until": zod.iso.datetime({"offset":true}).nullish()
});

export type AccountMute = zod.input<typeof AccountMute>;
export type AccountMuteOutput = zod.output<typeof AccountMute>;

export const duelRecordDrawsMin = 0;
export const duelRecordDrawsMax = 4294967295;

export const duelRecordLossesMin = 0;
export const duelRecordLossesMax = 4294967295;

export const duelRecordWinsMin = 0;
export const duelRecordWinsMax = 4294967295;



export const DuelRecord = zod.object({
  "draws": zod.int().min(duelRecordDrawsMin).max(duelRecordDrawsMax),
  "losses": zod.int().min(duelRecordLossesMin).max(duelRecordLossesMax),
  "wins": zod.int().min(duelRecordWinsMin).max(duelRecordWinsMax)
});

export type DuelRecord = zod.input<typeof DuelRecord>;
export type DuelRecordOutput = zod.output<typeof DuelRecord>;

export const playerDetailAdditionalCardsMin = 0;
export const playerDetailAdditionalCardsMax = 9007199254740991;

export const playerDetailCardKindsMin = 0;
export const playerDetailCardKindsMax = 4294967295;

export const playerDetailLevelMin = 0;
export const playerDetailLevelMax = 4294967295;

export const playerDetailOwnedItemsMin = 0;
export const playerDetailOwnedItemsMax = 4294967295;

export const playerDetailPlayerIdMin = 0;
export const playerDetailPlayerIdMax = 4294967295;

export const playerDetailPurchasedDecksMin = 0;
export const playerDetailPurchasedDecksMax = 4294967295;

export const playerDetailSlotMin = 0;
export const playerDetailSlotMax = 4294967295;

export const playerDetailTitleIdMin = 0;
export const playerDetailTitleIdMax = 4294967295;

export const playerDetailUploadedDecksMin = 0;
export const playerDetailUploadedDecksMax = 4294967295;



export const PlayerDetail = zod.object({
  "additional_cards": zod.int().min(playerDetailAdditionalCardsMin).max(playerDetailAdditionalCardsMax).describe('Additional card copies, excluding the starter deck. u16 card rows * u32 counts fit below 2^48, exactly in JS.'),
  "card_kinds": zod.int().min(playerDetailCardKindsMin).max(playerDetailCardKindsMax),
  "level": zod.int().min(playerDetailLevelMin).max(playerDetailLevelMax),
  "name": zod.string(),
  "online": zod.boolean().nullish().describe('Live presence is unknown without a running hub. Never infer it from saved accounts.'),
  "owned_items": zod.int().min(playerDetailOwnedItemsMin).max(playerDetailOwnedItemsMax),
  "player_id": zod.int().min(playerDetailPlayerIdMin).max(playerDetailPlayerIdMax),
  "purchased_decks": zod.int().min(playerDetailPurchasedDecksMin).max(playerDetailPurchasedDecksMax),
  "record": zod.union([DuelRecord,zod.null()]).optional(),
  "slot": zod.int().min(playerDetailSlotMin).max(playerDetailSlotMax),
  "title_id": zod.int().min(playerDetailTitleIdMin).max(playerDetailTitleIdMax),
  "uploaded_decks": zod.int().min(playerDetailUploadedDecksMin).max(playerDetailUploadedDecksMax)
});

export type PlayerDetail = zod.input<typeof PlayerDetail>;
export type PlayerDetailOutput = zod.output<typeof PlayerDetail>;

export const Role = zod.enum(['player', 'moderator', 'admin']);

export type Role = zod.input<typeof Role>;
export type RoleOutput = zod.output<typeof Role>;

export const AccountStanding = zod.union([zod.object({
  "kind": zod.enum(['active'])
}),zod.object({
  "kind": zod.enum(['restricted']),
  "reason": zod.string(),
  "until": zod.iso.datetime({"offset":true}).nullish()
}),zod.object({
  "kind": zod.enum(['banned']),
  "reason": zod.string(),
  "until": zod.iso.datetime({"offset":true}).nullish()
})]);

export type AccountStanding = zod.input<typeof AccountStanding>;
export type AccountStandingOutput = zod.output<typeof AccountStanding>;

export const accountDetailBpMin = 0;
export const accountDetailBpMax = 4294967295;

export const accountDetailDeviceCountMin = 0;
export const accountDetailDeviceCountMax = 4294967295;

export const accountDetailUserIdMin = 0;
export const accountDetailUserIdMax = 4294967295;



export const AccountDetail = zod.object({
  "bp": zod.int().min(accountDetailBpMin).max(accountDetailBpMax),
  "checked_at": zod.iso.datetime({"offset":true}),
  "country": zod.string().nullish(),
  "device_count": zod.int().min(accountDetailDeviceCountMin).max(accountDetailDeviceCountMax),
  "has_key": zod.boolean(),
  "login": zod.string().nullish(),
  "muted": zod.union([AccountMute,zod.null()]).optional(),
  "password_ready": zod.boolean().nullish(),
  "players": zod.array(PlayerDetail),
  "reset_pending": zod.boolean(),
  "role": Role,
  "standing": zod.union([AccountStanding,zod.null()]).optional(),
  "user_id": zod.int().min(accountDetailUserIdMin).max(accountDetailUserIdMax)
});

export type AccountDetail = zod.input<typeof AccountDetail>;
export type AccountDetailOutput = zod.output<typeof AccountDetail>;

export const accountEditRequestOneUserIdMin = 0;
export const accountEditRequestOneUserIdMax = 4294967295;

export const accountEditRequestTwoNameMax = 16;

export const accountEditRequestTwoUserIdMin = 0;
export const accountEditRequestTwoUserIdMax = 4294967295;

export const accountEditRequestThreeNameMax = 16;

export const accountEditRequestThreePlayerIdMin = 0;
export const accountEditRequestThreePlayerIdMax = 4294967295;

export const accountEditRequestThreeUserIdMin = 0;
export const accountEditRequestThreeUserIdMax = 4294967295;

export const accountEditRequestFourCountryMin = 2;
export const accountEditRequestFourCountryMax = 2;


export const accountEditRequestFourCountryRegExp = new RegExp('^[A-Za-z]{2}$');
export const accountEditRequestFourUserIdMin = 0;
export const accountEditRequestFourUserIdMax = 4294967295;



export const AccountEditRequest = zod.union([zod.object({
  "kind": zod.enum(['set_role']),
  "role": Role,
  "user_id": zod.int().min(accountEditRequestOneUserIdMin).max(accountEditRequestOneUserIdMax)
}),zod.object({
  "kind": zod.enum(['add_player']),
  "name": zod.string().min(1).max(accountEditRequestTwoNameMax),
  "user_id": zod.int().min(accountEditRequestTwoUserIdMin).max(accountEditRequestTwoUserIdMax)
}),zod.object({
  "kind": zod.enum(['rename_player']),
  "name": zod.string().min(1).max(accountEditRequestThreeNameMax),
  "player_id": zod.int().min(accountEditRequestThreePlayerIdMin).max(accountEditRequestThreePlayerIdMax),
  "user_id": zod.int().min(accountEditRequestThreeUserIdMin).max(accountEditRequestThreeUserIdMax)
}),zod.object({
  "country": zod.string().min(accountEditRequestFourCountryMin).max(accountEditRequestFourCountryMax).regex(accountEditRequestFourCountryRegExp).nullish().describe('Null or omitted: derive from the game\'s language. Other codes must occur in the client\'s table.'),
  "kind": zod.enum(['set_country']),
  "user_id": zod.int().min(accountEditRequestFourUserIdMin).max(accountEditRequestFourUserIdMax)
})]);

export type AccountEditRequest = zod.input<typeof AccountEditRequest>;
export type AccountEditRequestOutput = zod.output<typeof AccountEditRequest>;

export const accountEditResultPlayerIdMin = 0;
export const accountEditResultPlayerIdMax = 4294967295;

export const accountEditResultUserIdMin = 0;
export const accountEditResultUserIdMax = 4294967295;



export const AccountEditResult = zod.object({
  "message": zod.string(),
  "player_id": zod.int().min(accountEditResultPlayerIdMin).max(accountEditResultPlayerIdMax).nullish(),
  "request_id": zod.string(),
  "user_id": zod.int().min(accountEditResultUserIdMin).max(accountEditResultUserIdMax)
});

export type AccountEditResult = zod.input<typeof AccountEditResult>;
export type AccountEditResultOutput = zod.output<typeof AccountEditResult>;

export const AccountKeyMin = 20;
export const AccountKeyMax = 20;



export const AccountKey = zod.string().min(AccountKeyMin).max(AccountKeyMax);

export type AccountKey = zod.input<typeof AccountKey>;
export type AccountKeyOutput = zod.output<typeof AccountKey>;

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

export const ApiErrorCode = zod.enum(['invalid_request', 'unauthenticated', 'forbidden', 'invalid_origin', 'csrf_invalid', 'rate_limited', 'not_found', 'already_exists', 'request_conflict', 'persistence_failed', 'method_not_allowed', 'payload_too_large', 'unsupported_media_type', 'internal_error']);

export type ApiErrorCode = zod.input<typeof ApiErrorCode>;
export type ApiErrorCodeOutput = zod.output<typeof ApiErrorCode>;

export const ApiError = zod.object({
  "code": ApiErrorCode,
  "message": zod.string()
});

export type ApiError = zod.input<typeof ApiError>;
export type ApiErrorOutput = zod.output<typeof ApiError>;

export const bpCodeClaimUserIdMin = 0;
export const bpCodeClaimUserIdMax = 4294967295;



export const BpCodeClaim = zod.object({
  "at": zod.iso.datetime({"offset":true}).nullish(),
  "players": zod.array(zod.string()),
  "user_id": zod.int().min(bpCodeClaimUserIdMin).max(bpCodeClaimUserIdMax)
});

export type BpCodeClaim = zod.input<typeof BpCodeClaim>;
export type BpCodeClaimOutput = zod.output<typeof BpCodeClaim>;

export const bpCodeSummaryBpMin = 0;
export const bpCodeSummaryBpMax = 4294967295;

export const bpCodeSummaryUsesMin = 0;
export const bpCodeSummaryUsesMax = 4294967295;



export const BpCodeSummary = zod.object({
  "bp": zod.int().min(bpCodeSummaryBpMin).max(bpCodeSummaryBpMax),
  "claims": zod.array(BpCodeClaim),
  "code": zod.string(),
  "uses": zod.int().min(bpCodeSummaryUsesMin).max(bpCodeSummaryUsesMax).nullish()
});

export type BpCodeSummary = zod.input<typeof BpCodeSummary>;
export type BpCodeSummaryOutput = zod.output<typeof BpCodeSummary>;

export const BpCodeList = zod.object({
  "codes": zod.array(BpCodeSummary)
});

export type BpCodeList = zod.input<typeof BpCodeList>;
export type BpCodeListOutput = zod.output<typeof BpCodeList>;

export const CountryList = zod.object({
  "codes": zod.array(zod.string())
});

export type CountryList = zod.input<typeof CountryList>;
export type CountryListOutput = zod.output<typeof CountryList>;

export const createBpCodeRequestBpMax = 4294967295;

export const createBpCodeRequestCodeMin = 16;
export const createBpCodeRequestCodeMax = 16;


export const createBpCodeRequestCodeRegExp = new RegExp('^[A-Za-z0-9]{16}$');
export const createBpCodeRequestUsesMax = 4294967295;



export const CreateBpCodeRequest = zod.object({
  "bp": zod.int().min(1).max(createBpCodeRequestBpMax),
  "code": zod.string().min(createBpCodeRequestCodeMin).max(createBpCodeRequestCodeMax).regex(createBpCodeRequestCodeRegExp).nullish().describe('Omit for a generated code. Codes are case insensitive and stored in uppercase.'),
  "uses": zod.int().min(1).max(createBpCodeRequestUsesMax).nullish().describe('Number of accounts that can claim it. Omit for unlimited; each account can claim it once.')
});

export type CreateBpCodeRequest = zod.input<typeof CreateBpCodeRequest>;
export type CreateBpCodeRequestOutput = zod.output<typeof CreateBpCodeRequest>;

export const createBpCodeResultBpMin = 0;
export const createBpCodeResultBpMax = 4294967295;

export const createBpCodeResultUsesMin = 0;
export const createBpCodeResultUsesMax = 4294967295;



export const CreateBpCodeResult = zod.object({
  "bp": zod.int().min(createBpCodeResultBpMin).max(createBpCodeResultBpMax),
  "code": zod.string(),
  "message": zod.string(),
  "request_id": zod.string(),
  "uses": zod.int().min(createBpCodeResultUsesMin).max(createBpCodeResultUsesMax).nullish()
});

export type CreateBpCodeResult = zod.input<typeof CreateBpCodeResult>;
export type CreateBpCodeResultOutput = zod.output<typeof CreateBpCodeResult>;

export const createInviteRequestDaysMax = 4294967295;

export const createInviteRequestUsesMax = 4294967295;



export const CreateInviteRequest = zod.object({
  "days": zod.int().min(1).max(createInviteRequestDaysMax).nullish().describe('Days from the first successful creation. Omit for no expiry; retries keep the original expiry.'),
  "uses": zod.int().min(1).max(createInviteRequestUsesMax).nullish().describe('Number of sign-ups. Omit for unlimited.')
});

export type CreateInviteRequest = zod.input<typeof CreateInviteRequest>;
export type CreateInviteRequestOutput = zod.output<typeof CreateInviteRequest>;

export const createInviteResultUsesMin = 0;
export const createInviteResultUsesMax = 4294967295;



export const CreateInviteResult = zod.object({
  "code": zod.string(),
  "expires_at": zod.iso.datetime({"offset":true}).nullish(),
  "message": zod.string(),
  "request_id": zod.string(),
  "uses": zod.int().min(createInviteResultUsesMin).max(createInviteResultUsesMax).nullish()
});

export type CreateInviteResult = zod.input<typeof CreateInviteResult>;
export type CreateInviteResultOutput = zod.output<typeof CreateInviteResult>;

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

export const inviteClaimUserIdMin = 0;
export const inviteClaimUserIdMax = 4294967295;



export const InviteClaim = zod.object({
  "players": zod.array(zod.string()),
  "user_id": zod.int().min(inviteClaimUserIdMin).max(inviteClaimUserIdMax)
});

export type InviteClaim = zod.input<typeof InviteClaim>;
export type InviteClaimOutput = zod.output<typeof InviteClaim>;

export const InviteState = zod.enum(['usable', 'exhausted', 'expired']);

export type InviteState = zod.input<typeof InviteState>;
export type InviteStateOutput = zod.output<typeof InviteState>;

export const inviteSummaryUsesMin = 0;
export const inviteSummaryUsesMax = 4294967295;



export const InviteSummary = zod.object({
  "claims": zod.array(InviteClaim),
  "code": zod.string(),
  "expires_at": zod.iso.datetime({"offset":true}).nullish(),
  "issued_by": zod.string(),
  "state": InviteState,
  "uses": zod.int().min(inviteSummaryUsesMin).max(inviteSummaryUsesMax).nullish().describe('Remaining sign-ups, not the original limit. Null means unlimited.')
});

export type InviteSummary = zod.input<typeof InviteSummary>;
export type InviteSummaryOutput = zod.output<typeof InviteSummary>;

export const InviteList = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "invites": zod.array(InviteSummary)
});

export type InviteList = zod.input<typeof InviteList>;
export type InviteListOutput = zod.output<typeof InviteList>;

export const issueAccountCodeRequestOneCodeMin = 16;
export const issueAccountCodeRequestOneCodeMax = 19;


export const issueAccountCodeRequestOneCodeRegExp = new RegExp('^([A-Za-z0-9]{16}|[A-Za-z0-9]{4}(-[A-Za-z0-9]{4}){3})$');
export const issueAccountCodeRequestOneUserMin = 3;
export const issueAccountCodeRequestOneUserMax = 16;


export const issueAccountCodeRequestOneUserRegExp = new RegExp('^[A-Za-z0-9]+$');
export const issueAccountCodeRequestTwoCodeMin = 16;
export const issueAccountCodeRequestTwoCodeMax = 19;


export const issueAccountCodeRequestTwoCodeRegExp = new RegExp('^([A-Za-z0-9]{16}|[A-Za-z0-9]{4}(-[A-Za-z0-9]{4}){3})$');
export const issueAccountCodeRequestTwoUserIdMin = 0;
export const issueAccountCodeRequestTwoUserIdMax = 4294967295;



export const IssueAccountCodeRequest = zod.union([zod.object({
  "code": zod.string().min(issueAccountCodeRequestOneCodeMin).max(issueAccountCodeRequestOneCodeMax).regex(issueAccountCodeRequestOneCodeRegExp),
  "kind": zod.enum(['create_account']),
  "user": zod.string().min(issueAccountCodeRequestOneUserMin).max(issueAccountCodeRequestOneUserMax).regex(issueAccountCodeRequestOneUserRegExp)
}),zod.object({
  "code": zod.string().min(issueAccountCodeRequestTwoCodeMin).max(issueAccountCodeRequestTwoCodeMax).regex(issueAccountCodeRequestTwoCodeRegExp),
  "kind": zod.enum(['reset_password']),
  "user_id": zod.int().min(issueAccountCodeRequestTwoUserIdMin).max(issueAccountCodeRequestTwoUserIdMax)
})]);

export type IssueAccountCodeRequest = zod.input<typeof IssueAccountCodeRequest>;
export type IssueAccountCodeRequestOutput = zod.output<typeof IssueAccountCodeRequest>;

export const keyAccountRequestOneNamesMax = 4;

export const keyAccountRequestTwoUserIdMin = 0;
export const keyAccountRequestTwoUserIdMax = 4294967295;



export const KeyAccountRequest = zod.union([zod.object({
  "kind": zod.enum(['create']),
  "names": zod.array(zod.string()).max(keyAccountRequestOneNamesMax)
}).describe('No names: use the host\'s starting players, made unique. Explicit names must be available.'),zod.object({
  "kind": zod.enum(['reveal']),
  "user_id": zod.int().min(keyAccountRequestTwoUserIdMin).max(keyAccountRequestTwoUserIdMax)
}).describe('Reveal the existing key, or assign one. The key allows passwordless game access.')]);

export type KeyAccountRequest = zod.input<typeof KeyAccountRequest>;
export type KeyAccountRequestOutput = zod.output<typeof KeyAccountRequest>;

export const keyPlayerPlayerIdMin = 0;
export const keyPlayerPlayerIdMax = 4294967295;



export const KeyPlayer = zod.object({
  "name": zod.string(),
  "player_id": zod.int().min(keyPlayerPlayerIdMin).max(keyPlayerPlayerIdMax)
});

export type KeyPlayer = zod.input<typeof KeyPlayer>;
export type KeyPlayerOutput = zod.output<typeof KeyPlayer>;

export const keyAccountResultUserIdMin = 0;
export const keyAccountResultUserIdMax = 4294967295;



export const KeyAccountResult = zod.object({
  "key": AccountKey,
  "players": zod.array(KeyPlayer),
  "request_id": zod.string(),
  "user_id": zod.int().min(keyAccountResultUserIdMin).max(keyAccountResultUserIdMax)
});

export type KeyAccountResult = zod.input<typeof KeyAccountResult>;
export type KeyAccountResultOutput = zod.output<typeof KeyAccountResult>;

export const removeBpCodeRequestCodeMin = 16;
export const removeBpCodeRequestCodeMax = 16;


export const removeBpCodeRequestCodeRegExp = new RegExp('^[A-Za-z0-9]{16}$');


export const RemoveBpCodeRequest = zod.object({
  "code": zod.string().min(removeBpCodeRequestCodeMin).max(removeBpCodeRequestCodeMax).regex(removeBpCodeRequestCodeRegExp)
});

export type RemoveBpCodeRequest = zod.input<typeof RemoveBpCodeRequest>;
export type RemoveBpCodeRequestOutput = zod.output<typeof RemoveBpCodeRequest>;

export const RemoveBpCodeResult = zod.object({
  "message": zod.string(),
  "removed": zod.string(),
  "request_id": zod.string()
});

export type RemoveBpCodeResult = zod.input<typeof RemoveBpCodeResult>;
export type RemoveBpCodeResultOutput = zod.output<typeof RemoveBpCodeResult>;

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


export const issueAccountCodeHeaderIdempotencyKeyMin = 16;
export const issueAccountCodeHeaderIdempotencyKeyMax = 128;


export const issueAccountCodeHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const IssueAccountCodeHeader = zod.object({
  "Idempotency-Key": zod.string().min(issueAccountCodeHeaderIdempotencyKeyMin).max(issueAccountCodeHeaderIdempotencyKeyMax).regex(issueAccountCodeHeaderIdempotencyKeyRegExp).describe('Retain this ID and the same generated code on retry. Codes never appear in durable receipts.')
})

export const IssueAccountCodeBody = IssueAccountCodeRequest

export const IssueAccountCode200Response = AccountCodeResponse

export const IssueAccountCode400Response = ApiError

export const IssueAccountCode401Response = ApiError

export const IssueAccountCode403Response = ApiError

export const IssueAccountCode404Response = ApiError

export const IssueAccountCode405Response = ApiError

export const IssueAccountCode409Response = ApiError

export const IssueAccountCode413Response = ApiError

export const IssueAccountCode415Response = ApiError

export const IssueAccountCode500Response = ApiError

export const IssueAccountCode503Response = ApiError


export const editAccountHeaderIdempotencyKeyMin = 16;
export const editAccountHeaderIdempotencyKeyMax = 128;


export const editAccountHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const EditAccountHeader = zod.object({
  "Idempotency-Key": zod.string().min(editAccountHeaderIdempotencyKeyMin).max(editAccountHeaderIdempotencyKeyMax).regex(editAccountHeaderIdempotencyKeyRegExp).describe('One ID per change. Retain it on an uncertain outcome; retries never reapply a committed edit.')
})

export const EditAccountBody = AccountEditRequest

export const EditAccount200Response = AccountEditResult

export const EditAccount400Response = ApiError

export const EditAccount401Response = ApiError

export const EditAccount403Response = ApiError

export const EditAccount404Response = ApiError

export const EditAccount405Response = ApiError

export const EditAccount409Response = ApiError

export const EditAccount413Response = ApiError

export const EditAccount415Response = ApiError

export const EditAccount500Response = ApiError

export const EditAccount503Response = ApiError


export const manageAccountKeyHeaderIdempotencyKeyMin = 16;
export const manageAccountKeyHeaderIdempotencyKeyMax = 128;


export const manageAccountKeyHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const ManageAccountKeyHeader = zod.object({
  "Idempotency-Key": zod.string().min(manageAccountKeyHeaderIdempotencyKeyMin).max(manageAccountKeyHeaderIdempotencyKeyMax).regex(manageAccountKeyHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A key is returned only after its audit is committed; never stored in a receipt.')
})

export const ManageAccountKeyBody = KeyAccountRequest

export const ManageAccountKey200Response = KeyAccountResult

export const ManageAccountKey400Response = ApiError

export const ManageAccountKey401Response = ApiError

export const ManageAccountKey403Response = ApiError

export const ManageAccountKey404Response = ApiError

export const ManageAccountKey405Response = ApiError

export const ManageAccountKey409Response = ApiError

export const ManageAccountKey413Response = ApiError

export const ManageAccountKey415Response = ApiError

export const ManageAccountKey500Response = ApiError

export const ManageAccountKey503Response = ApiError


export const getAccountPathUserIdMin = 0;
export const getAccountPathUserIdMax = 4294967295;



export const GetAccountParams = zod.object({
  "user_id": zod.int().min(getAccountPathUserIdMin).max(getAccountPathUserIdMax).describe('Stable account number')
})

export const GetAccount200Response = AccountDetail

export const GetAccount400Response = ApiError

export const GetAccount401Response = ApiError

export const GetAccount403Response = ApiError

export const GetAccount404Response = ApiError

export const GetAccount405Response = ApiError

export const GetAccount413Response = ApiError

export const GetAccount415Response = ApiError

export const GetAccount500Response = ApiError

export const GetAccount503Response = ApiError


export const ListBpCodes200Response = BpCodeList

export const ListBpCodes400Response = ApiError

export const ListBpCodes401Response = ApiError

export const ListBpCodes403Response = ApiError

export const ListBpCodes405Response = ApiError

export const ListBpCodes413Response = ApiError

export const ListBpCodes415Response = ApiError

export const ListBpCodes500Response = ApiError


export const createBpCodeHeaderIdempotencyKeyMin = 16;
export const createBpCodeHeaderIdempotencyKeyMax = 128;


export const createBpCodeHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const CreateBpCodeHeader = zod.object({
  "Idempotency-Key": zod.string().min(createBpCodeHeaderIdempotencyKeyMin).max(createBpCodeHeaderIdempotencyKeyMax).regex(createBpCodeHeaderIdempotencyKeyRegExp).describe('One ID per change. Retain it on an uncertain outcome; retries return the original code.')
})

export const CreateBpCodeBody = CreateBpCodeRequest

export const CreateBpCode200Response = CreateBpCodeResult

export const CreateBpCode400Response = ApiError

export const CreateBpCode401Response = ApiError

export const CreateBpCode403Response = ApiError

export const CreateBpCode405Response = ApiError

export const CreateBpCode409Response = ApiError

export const CreateBpCode413Response = ApiError

export const CreateBpCode415Response = ApiError

export const CreateBpCode500Response = ApiError

export const CreateBpCode503Response = ApiError


export const removeBpCodeHeaderIdempotencyKeyMin = 16;
export const removeBpCodeHeaderIdempotencyKeyMax = 128;


export const removeBpCodeHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const RemoveBpCodeHeader = zod.object({
  "Idempotency-Key": zod.string().min(removeBpCodeHeaderIdempotencyKeyMin).max(removeBpCodeHeaderIdempotencyKeyMax).regex(removeBpCodeHeaderIdempotencyKeyRegExp).describe('One ID per removal. Retain it on an uncertain outcome; do not generate a new one on retry.')
})

export const RemoveBpCodeBody = RemoveBpCodeRequest

export const RemoveBpCode200Response = RemoveBpCodeResult

export const RemoveBpCode400Response = ApiError

export const RemoveBpCode401Response = ApiError

export const RemoveBpCode403Response = ApiError

export const RemoveBpCode404Response = ApiError

export const RemoveBpCode405Response = ApiError

export const RemoveBpCode409Response = ApiError

export const RemoveBpCode413Response = ApiError

export const RemoveBpCode415Response = ApiError

export const RemoveBpCode500Response = ApiError

export const RemoveBpCode503Response = ApiError


export const ListCountries200Response = CountryList

export const ListCountries400Response = ApiError

export const ListCountries401Response = ApiError

export const ListCountries403Response = ApiError

export const ListCountries405Response = ApiError

export const ListCountries413Response = ApiError

export const ListCountries415Response = ApiError

export const ListCountries500Response = ApiError

export const ListCountries503Response = ApiError


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


export const ListInvites200Response = InviteList

export const ListInvites400Response = ApiError

export const ListInvites401Response = ApiError

export const ListInvites403Response = ApiError

export const ListInvites405Response = ApiError

export const ListInvites413Response = ApiError

export const ListInvites415Response = ApiError

export const ListInvites500Response = ApiError

export const ListInvites503Response = ApiError


export const createInviteHeaderIdempotencyKeyMin = 16;
export const createInviteHeaderIdempotencyKeyMax = 128;


export const createInviteHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const CreateInviteHeader = zod.object({
  "Idempotency-Key": zod.string().min(createInviteHeaderIdempotencyKeyMin).max(createInviteHeaderIdempotencyKeyMax).regex(createInviteHeaderIdempotencyKeyRegExp).describe('One ID per change. Retain it on an uncertain outcome; retries return the original invite and expiry.')
})

export const CreateInviteBody = CreateInviteRequest

export const CreateInvite200Response = CreateInviteResult

export const CreateInvite400Response = ApiError

export const CreateInvite401Response = ApiError

export const CreateInvite403Response = ApiError

export const CreateInvite405Response = ApiError

export const CreateInvite409Response = ApiError

export const CreateInvite413Response = ApiError

export const CreateInvite415Response = ApiError

export const CreateInvite500Response = ApiError

export const CreateInvite503Response = ApiError


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
