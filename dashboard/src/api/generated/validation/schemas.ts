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

export const banlistChoiceIdMin = 0;
export const banlistChoiceIdMax = 4294967295;



export const BanlistChoice = zod.object({
  "id": zod.int().min(banlistChoiceIdMin).max(banlistChoiceIdMax),
  "name": zod.string()
});

export type BanlistChoice = zod.input<typeof BanlistChoice>;
export type BanlistChoiceOutput = zod.output<typeof BanlistChoice>;

export const AccountBanlist = BanlistChoice.and(zod.object({
  "chosen": zod.boolean().describe('True when the account (or the staff) chose it; false when it is the server\'s `login-regulation`.')
})).describe('The banlist an account\'s game logs in with.');

export type AccountBanlist = zod.input<typeof AccountBanlist>;
export type AccountBanlistOutput = zod.output<typeof AccountBanlist>;

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
  "banlist": AccountBanlist.describe('The banlist the account\'s game logs in with (its own choice with `/banlist`, a staff override, or the server\'s).'),
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
  "muted": zod.union([AccountMute,zod.null()]).optional(),
  "players": zod.array(PlayerSummary),
  "role": Role,
  "standing": zod.union([AccountStanding,zod.null()]).optional(),
  "user_id": zod.int().min(accountSummaryUserIdMin).max(accountSummaryUserIdMax)
}).describe('A projection of existing account state: no keys, password hashes, device tokens or invented dates/presence.');

export type AccountSummary = zod.input<typeof AccountSummary>;
export type AccountSummaryOutput = zod.output<typeof AccountSummary>;

export const AccountList = zod.object({
  "accounts": zod.array(AccountSummary)
});

export type AccountList = zod.input<typeof AccountList>;
export type AccountListOutput = zod.output<typeof AccountList>;

export const ApiErrorCode = zod.enum(['invalid_request', 'unauthenticated', 'forbidden', 'invalid_origin', 'csrf_invalid', 'rate_limited', 'not_found', 'already_exists', 'request_conflict', 'persistence_failed', 'reauthentication_required', 'invalid_code', 'mfa_enrollment_required', 'method_not_allowed', 'payload_too_large', 'unsupported_media_type', 'internal_error']);

export type ApiErrorCode = zod.input<typeof ApiErrorCode>;
export type ApiErrorCodeOutput = zod.output<typeof ApiErrorCode>;

export const ApiError = zod.object({
  "code": ApiErrorCode,
  "message": zod.string()
});

export type ApiError = zod.input<typeof ApiError>;
export type ApiErrorOutput = zod.output<typeof ApiError>;

export const AuditEntry = zod.object({
  "actor": zod.string(),
  "after": zod.string().nullish(),
  "at": zod.iso.datetime({"offset":true}),
  "before": zod.string().nullish().describe('Pretty-printed JSON for display as text, not executable HTML or a business-operation response.'),
  "detail": zod.string().nullish(),
  "id": zod.string().describe('Decimal SQLite row ID. Never convert this to a JavaScript number.'),
  "operation": zod.string(),
  "origin": zod.string(),
  "target": zod.string()
});

export type AuditEntry = zod.input<typeof AuditEntry>;
export type AuditEntryOutput = zod.output<typeof AuditEntry>;

export const AuditPage = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "entries": zod.array(AuditEntry),
  "next_before": zod.string().nullish()
});

export type AuditPage = zod.input<typeof AuditPage>;
export type AuditPageOutput = zod.output<typeof AuditPage>;

export const BackupEntry = zod.object({
  "at": zod.iso.datetime({"offset":true}).nullish(),
  "bytes": zod.string(),
  "name": zod.string()
});

export type BackupEntry = zod.input<typeof BackupEntry>;
export type BackupEntryOutput = zod.output<typeof BackupEntry>;

export const BackupFile = zod.instanceof(Blob);

export type BackupFile = zod.input<typeof BackupFile>;
export type BackupFileOutput = zod.output<typeof BackupFile>;

export const BackupHealth = zod.union([zod.object({
  "kind": zod.enum(['none'])
}),zod.object({
  "at": zod.iso.datetime({"offset":true}),
  "kind": zod.enum(['available']),
  "name": zod.string()
}),zod.object({
  "kind": zod.enum(['unavailable']),
  "message": zod.string()
})]);

export type BackupHealth = zod.input<typeof BackupHealth>;
export type BackupHealthOutput = zod.output<typeof BackupHealth>;

export const BackupList = zod.object({
  "backups": zod.array(BackupEntry),
  "checked_at": zod.iso.datetime({"offset":true}),
  "downloads_allowed": zod.boolean()
});

export type BackupList = zod.input<typeof BackupList>;
export type BackupListOutput = zod.output<typeof BackupList>;

export const BanlistChoices = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "choices": zod.array(BanlistChoice),
  "server_default": BanlistChoice.describe('What an account without a choice logs in with.')
});

export type BanlistChoices = zod.input<typeof BanlistChoices>;
export type BanlistChoicesOutput = zod.output<typeof BanlistChoices>;

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

export const cardChoiceArtMin = 0;
export const cardChoiceArtMax = 4294967295;

export const cardChoiceKonamiIdMin = 0;

export const cardChoiceRowMin = 0;



export const CardChoice = zod.object({
  "art": zod.int().min(cardChoiceArtMin).max(cardChoiceArtMax),
  "description": zod.string(),
  "image_url": zod.string().nullish(),
  "konami_id": zod.int().min(cardChoiceKonamiIdMin),
  "name": zod.string(),
  "row": zod.int().min(cardChoiceRowMin),
  "thumbnail_url": zod.string().nullish()
});

export type CardChoice = zod.input<typeof CardChoice>;
export type CardChoiceOutput = zod.output<typeof CardChoice>;

export const CardImageSize = zod.enum(['mini', 'full']);

export type CardImageSize = zod.input<typeof CardImageSize>;
export type CardImageSizeOutput = zod.output<typeof CardImageSize>;

export const CardPng = zod.instanceof(Blob);

export type CardPng = zod.input<typeof CardPng>;
export type CardPngOutput = zod.output<typeof CardPng>;

export const codeRequestCodeMin = 6;
export const codeRequestCodeMax = 32;



export const CodeRequest = zod.object({
  "code": zod.string().min(codeRequestCodeMin).max(codeRequestCodeMax).describe('An authenticator code (six digits) or an unused recovery code.')
});

export type CodeRequest = zod.input<typeof CodeRequest>;
export type CodeRequestOutput = zod.output<typeof CodeRequest>;

export const CountryList = zod.object({
  "codes": zod.array(zod.string())
});

export type CountryList = zod.input<typeof CountryList>;
export type CountryListOutput = zod.output<typeof CountryList>;

export const CreateBackupRequest = zod.object({

});

export type CreateBackupRequest = zod.input<typeof CreateBackupRequest>;
export type CreateBackupRequestOutput = zod.output<typeof CreateBackupRequest>;

export const EffectState = zod.enum(['pending', 'completed', 'interrupted', 'failed']);

export type EffectState = zod.input<typeof EffectState>;
export type EffectStateOutput = zod.output<typeof EffectState>;

export const CreateBackupResult = zod.object({
  "message": zod.string(),
  "name": zod.string(),
  "request_id": zod.string()
});

export type CreateBackupResult = zod.input<typeof CreateBackupResult>;
export type CreateBackupResultOutput = zod.output<typeof CreateBackupResult>;

export const CreateBackupResponse = zod.object({
  "available": zod.boolean(),
  "backup": EffectState,
  "result": CreateBackupResult
});

export type CreateBackupResponse = zod.input<typeof CreateBackupResponse>;
export type CreateBackupResponseOutput = zod.output<typeof CreateBackupResponse>;

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

export const deleteAccountRequestConfirmUserIdMin = 0;
export const deleteAccountRequestConfirmUserIdMax = 4294967295;

export const deleteAccountRequestUserIdMin = 0;
export const deleteAccountRequestUserIdMax = 4294967295;



export const DeleteAccountRequest = zod.object({
  "confirm_user_id": zod.int().min(deleteAccountRequestConfirmUserIdMin).max(deleteAccountRequestConfirmUserIdMax).describe('Must match user_id. The dashboard asks the administrator to type this account number.'),
  "user_id": zod.int().min(deleteAccountRequestUserIdMin).max(deleteAccountRequestUserIdMax)
});

export type DeleteAccountRequest = zod.input<typeof DeleteAccountRequest>;
export type DeleteAccountRequestOutput = zod.output<typeof DeleteAccountRequest>;

export const deleteAccountResultPlayersMin = 0;
export const deleteAccountResultPlayersMax = 4294967295;

export const deleteAccountResultUserIdMin = 0;
export const deleteAccountResultUserIdMax = 4294967295;



export const DeleteAccountResult = zod.object({
  "message": zod.string(),
  "players": zod.int().min(deleteAccountResultPlayersMin).max(deleteAccountResultPlayersMax),
  "request_id": zod.string(),
  "user_id": zod.int().min(deleteAccountResultUserIdMin).max(deleteAccountResultUserIdMax)
});

export type DeleteAccountResult = zod.input<typeof DeleteAccountResult>;
export type DeleteAccountResultOutput = zod.output<typeof DeleteAccountResult>;

export const DeleteAccountResponse = zod.object({
  "disconnect": zod.union([EffectState,zod.null()]).optional(),
  "result": DeleteAccountResult
});

export type DeleteAccountResponse = zod.input<typeof DeleteAccountResponse>;
export type DeleteAccountResponseOutput = zod.output<typeof DeleteAccountResponse>;

export const downloadBackupRequestNameMax = 96;


export const downloadBackupRequestNameRegExp = new RegExp('^[A-Za-z0-9][A-Za-z0-9._-]*$');


export const DownloadBackupRequest = zod.object({
  "name": zod.string().min(1).max(downloadBackupRequestNameMax).regex(downloadBackupRequestNameRegExp)
});

export type DownloadBackupRequest = zod.input<typeof DownloadBackupRequest>;
export type DownloadBackupRequestOutput = zod.output<typeof DownloadBackupRequest>;

export const EmptyRequest = zod.object({

});

export type EmptyRequest = zod.input<typeof EmptyRequest>;
export type EmptyRequestOutput = zod.output<typeof EmptyRequest>;

export const recipeChoiceCardCountMin = 0;
export const recipeChoiceCardCountMax = 4294967295;

export const recipeChoiceDeckIdMin = 0;
export const recipeChoiceDeckIdMax = 4294967295;



export const RecipeChoice = zod.object({
  "card_count": zod.int().min(recipeChoiceCardCountMin).max(recipeChoiceCardCountMax),
  "deck_id": zod.int().min(recipeChoiceDeckIdMin).max(recipeChoiceDeckIdMax),
  "name": zod.string()
});

export type RecipeChoice = zod.input<typeof RecipeChoice>;
export type RecipeChoiceOutput = zod.output<typeof RecipeChoice>;

export const titleChoiceTitleIdMin = 0;



export const TitleChoice = zod.object({
  "name": zod.string(),
  "title_id": zod.int().min(titleChoiceTitleIdMin)
});

export type TitleChoice = zod.input<typeof TitleChoice>;
export type TitleChoiceOutput = zod.output<typeof TitleChoice>;

export const GiftChoices = zod.object({
  "cards": zod.array(CardChoice),
  "more_cards": zod.boolean(),
  "more_recipes": zod.boolean(),
  "more_titles": zod.boolean(),
  "recipes": zod.array(RecipeChoice),
  "titles": zod.array(TitleChoice)
});

export type GiftChoices = zod.input<typeof GiftChoices>;
export type GiftChoicesOutput = zod.output<typeof GiftChoices>;

export const giftOutcomeOneAddedMin = 0;
export const giftOutcomeOneAddedMax = 255;

export const giftOutcomeOneRowMin = 0;

export const giftOutcomeOneTotalMin = 0;
export const giftOutcomeOneTotalMax = 255;

export const giftOutcomeTwoDeckIdMin = 0;
export const giftOutcomeTwoDeckIdMax = 4294967295;

export const giftOutcomeThreeTitleIdMin = 0;



export const GiftOutcome = zod.union([zod.object({
  "added": zod.int().min(giftOutcomeOneAddedMin).max(giftOutcomeOneAddedMax),
  "kind": zod.enum(['cards']),
  "row": zod.int().min(giftOutcomeOneRowMin),
  "total": zod.int().min(giftOutcomeOneTotalMin).max(giftOutcomeOneTotalMax)
}),zod.object({
  "deck_id": zod.int().min(giftOutcomeTwoDeckIdMin).max(giftOutcomeTwoDeckIdMax),
  "include_cards": zod.boolean(),
  "kind": zod.enum(['recipe'])
}),zod.object({
  "already_owned": zod.boolean(),
  "kind": zod.enum(['title']),
  "title_id": zod.int().min(giftOutcomeThreeTitleIdMin)
})]);

export type GiftOutcome = zod.input<typeof GiftOutcome>;
export type GiftOutcomeOutput = zod.output<typeof GiftOutcome>;

export const giftRequestOneCountMax = 255;

export const giftRequestOnePlayerIdMin = 0;
export const giftRequestOnePlayerIdMax = 4294967295;

export const giftRequestOneRowMin = 0;

export const giftRequestTwoDeckIdMin = 0;
export const giftRequestTwoDeckIdMax = 4294967295;

export const giftRequestTwoPlayerIdMin = 0;
export const giftRequestTwoPlayerIdMax = 4294967295;

export const giftRequestThreePlayerIdMin = 0;
export const giftRequestThreePlayerIdMax = 4294967295;

export const giftRequestThreeTitleIdMin = 0;



export const GiftRequest = zod.union([zod.object({
  "count": zod.int().min(1).max(giftRequestOneCountMax),
  "kind": zod.enum(['cards']),
  "player_id": zod.int().min(giftRequestOnePlayerIdMin).max(giftRequestOnePlayerIdMax),
  "row": zod.int().min(giftRequestOneRowMin)
}),zod.object({
  "deck_id": zod.int().min(giftRequestTwoDeckIdMin).max(giftRequestTwoDeckIdMax),
  "include_cards": zod.boolean(),
  "kind": zod.enum(['recipe']),
  "player_id": zod.int().min(giftRequestTwoPlayerIdMin).max(giftRequestTwoPlayerIdMax)
}),zod.object({
  "kind": zod.enum(['title']),
  "player_id": zod.int().min(giftRequestThreePlayerIdMin).max(giftRequestThreePlayerIdMax),
  "title_id": zod.int().min(giftRequestThreeTitleIdMin)
})]);

export type GiftRequest = zod.input<typeof GiftRequest>;
export type GiftRequestOutput = zod.output<typeof GiftRequest>;

export const giftResultPlayerIdMin = 0;
export const giftResultPlayerIdMax = 4294967295;

export const giftResultUserIdMin = 0;
export const giftResultUserIdMax = 4294967295;



export const GiftResult = zod.object({
  "gift": GiftOutcome,
  "message": zod.string(),
  "player_id": zod.int().min(giftResultPlayerIdMin).max(giftResultPlayerIdMax),
  "request_id": zod.string(),
  "user_id": zod.int().min(giftResultUserIdMin).max(giftResultUserIdMax)
});

export type GiftResult = zod.input<typeof GiftResult>;
export type GiftResultOutput = zod.output<typeof GiftResult>;

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

export const Status = zod.enum(['open', 'handled', 'dismissed']).describe('Where a report stands.');

export type Status = zod.input<typeof Status>;
export type StatusOutput = zod.output<typeof Status>;

export const handleReportRequestNoteMax = 300;

export const handleReportRequestReportIdMin = 0;
export const handleReportRequestReportIdMax = 4294967295;



export const HandleReportRequest = zod.object({
  "note": zod.string().max(handleReportRequestNoteMax).optional().describe('What was done, shown with the report (optional).'),
  "report_id": zod.int().min(handleReportRequestReportIdMin).max(handleReportRequestReportIdMax),
  "status": Status.describe('handled or dismissed.')
});

export type HandleReportRequest = zod.input<typeof HandleReportRequest>;
export type HandleReportRequestOutput = zod.output<typeof HandleReportRequest>;

export const reportIdMin = 0;
export const reportIdMax = 4294967295;

export const reportReporterIdMin = 0;

export const reportTargetIdMin = 0;



export const Report = zod.object({
  "at": zod.string().describe('RFC 3339 UTC, to the second.'),
  "handled_at": zod.string().nullish(),
  "handled_by": zod.string().nullish(),
  "id": zod.int().min(reportIdMin).max(reportIdMax),
  "note": zod.string().nullish(),
  "reason": zod.string(),
  "reporter": zod.string(),
  "reporter_id": zod.int().min(reportReporterIdMin),
  "status": Status,
  "target": zod.string(),
  "target_id": zod.int().min(reportTargetIdMin)
});

export type Report = zod.input<typeof Report>;
export type ReportOutput = zod.output<typeof Report>;

export const reportViewTwoReporterUserIdMin = 0;
export const reportViewTwoReporterUserIdMax = 4294967295;

export const reportViewTwoTargetUserIdMin = 0;
export const reportViewTwoTargetUserIdMax = 4294967295;



export const ReportView = Report.and(zod.object({
  "reporter_user_id": zod.int().min(reportViewTwoReporterUserIdMin).max(reportViewTwoReporterUserIdMax).nullish().describe('The account of the reporting player (none once deleted).'),
  "target_user_id": zod.int().min(reportViewTwoTargetUserIdMin).max(reportViewTwoTargetUserIdMax).nullish()
})).describe('A report with the accounts of the two players, for the page\'s links.');

export type ReportView = zod.input<typeof ReportView>;
export type ReportViewOutput = zod.output<typeof ReportView>;

export const HandleReportResult = zod.object({
  "message": zod.string(),
  "report": ReportView
});

export type HandleReportResult = zod.input<typeof HandleReportResult>;
export type HandleReportResultOutput = zod.output<typeof HandleReportResult>;

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

export const LogLevel = zod.enum(['error', 'warn', 'info', 'debug']);

export type LogLevel = zod.input<typeof LogLevel>;
export type LogLevelOutput = zod.output<typeof LogLevel>;

export const LogEntry = zod.object({
  "at": zod.iso.datetime({"offset":true}),
  "level": LogLevel,
  "text": zod.string()
});

export type LogEntry = zod.input<typeof LogEntry>;
export type LogEntryOutput = zod.output<typeof LogEntry>;

export const logPageRetentionLinesMin = 0;
export const logPageRetentionLinesMax = 4294967295;



export const LogPage = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "lines": zod.array(LogEntry),
  "retention_lines": zod.int().min(logPageRetentionLinesMin).max(logPageRetentionLinesMax)
});

export type LogPage = zod.input<typeof LogPage>;
export type LogPageOutput = zod.output<typeof LogPage>;

export const staffSessionRecoveryCodesRemainingMin = 0;
export const staffSessionRecoveryCodesRemainingMax = 100;

export const staffSessionUserIdMin = 0;
export const staffSessionUserIdMax = 4294967295;



export const StaffSession = zod.object({
  "csrf_token": zod.string(),
  "expires_at": zod.iso.datetime({"offset":true}).describe('Idle expiry, refreshed by an authenticated request; capped by the absolute session lifetime.'),
  "limited": zod.boolean().describe('An enrollment-only session: every route but enrollment, sign-out and this session answers `mfa_enrollment_required`.'),
  "mfa_enabled": zod.boolean().describe('The account has an active authenticator (TOTP).'),
  "mfa_recent_until": zod.iso.datetime({"offset":true}).nullish().describe('When the authenticator\'s recent check ends; none: sensitive operations ask for a code first.'),
  "mfa_required": zod.boolean().describe('The server requires an authenticator of every staff account (the default policy).'),
  "recovery_codes_remaining": zod.int().min(staffSessionRecoveryCodesRemainingMin).max(staffSessionRecoveryCodesRemainingMax).describe('Unused recovery codes left.'),
  "role": Role,
  "user": zod.string(),
  "user_id": zod.int().min(staffSessionUserIdMin).max(staffSessionUserIdMax)
});

export type StaffSession = zod.input<typeof StaffSession>;
export type StaffSessionOutput = zod.output<typeof StaffSession>;

export const MfaEnrolled = zod.object({
  "recovery_codes": zod.array(zod.string()),
  "session": StaffSession
}).describe('An authenticator was activated. The recovery codes are shown once and are kept only as digests.');

export type MfaEnrolled = zod.input<typeof MfaEnrolled>;
export type MfaEnrolledOutput = zod.output<typeof MfaEnrolled>;

export const MfaEnrollmentStart = zod.object({
  "account": zod.string(),
  "issuer": zod.string(),
  "otpauth_uri": zod.string(),
  "secret": zod.string()
}).describe('The seed of a new authenticator, shown to its owner once. The QR code is the `otpauth_uri`.');

export type MfaEnrollmentStart = zod.input<typeof MfaEnrollmentStart>;
export type MfaEnrollmentStartOutput = zod.output<typeof MfaEnrollmentStart>;

export const moderationRequestOneUserIdMin = 0;
export const moderationRequestOneUserIdMax = 4294967295;

export const moderationRequestTwoDaysMax = 4294967295;

export const moderationRequestTwoReasonMax = 200;

export const moderationRequestTwoUserIdMin = 0;
export const moderationRequestTwoUserIdMax = 4294967295;

export const moderationRequestThreeDaysMax = 4294967295;

export const moderationRequestThreeReasonMax = 200;

export const moderationRequestThreeUserIdMin = 0;
export const moderationRequestThreeUserIdMax = 4294967295;

export const moderationRequestFourUserIdMin = 0;
export const moderationRequestFourUserIdMax = 4294967295;

export const moderationRequestFiveMinutesMax = 4294967295;

export const moderationRequestFiveReasonMax = 200;

export const moderationRequestFiveUserIdMin = 0;
export const moderationRequestFiveUserIdMax = 4294967295;

export const moderationRequestSixUserIdMin = 0;
export const moderationRequestSixUserIdMax = 4294967295;

export const moderationRequestSevenUserIdMin = 0;
export const moderationRequestSevenUserIdMax = 4294967295;



export const ModerationRequest = zod.union([zod.object({
  "kind": zod.enum(['kick']),
  "user_id": zod.int().min(moderationRequestOneUserIdMin).max(moderationRequestOneUserIdMax)
}),zod.object({
  "days": zod.int().min(1).max(moderationRequestTwoDaysMax).nullish(),
  "kind": zod.enum(['ban']),
  "reason": zod.string().max(moderationRequestTwoReasonMax),
  "user_id": zod.int().min(moderationRequestTwoUserIdMin).max(moderationRequestTwoUserIdMax)
}),zod.object({
  "days": zod.int().min(1).max(moderationRequestThreeDaysMax).nullish(),
  "kind": zod.enum(['restrict']),
  "reason": zod.string().max(moderationRequestThreeReasonMax),
  "user_id": zod.int().min(moderationRequestThreeUserIdMin).max(moderationRequestThreeUserIdMax)
}),zod.object({
  "kind": zod.enum(['lift']),
  "user_id": zod.int().min(moderationRequestFourUserIdMin).max(moderationRequestFourUserIdMax)
}),zod.object({
  "kind": zod.enum(['mute']),
  "minutes": zod.int().min(1).max(moderationRequestFiveMinutesMax).nullish(),
  "reason": zod.string().max(moderationRequestFiveReasonMax),
  "user_id": zod.int().min(moderationRequestFiveUserIdMin).max(moderationRequestFiveUserIdMax)
}),zod.object({
  "kind": zod.enum(['unmute']),
  "user_id": zod.int().min(moderationRequestSixUserIdMin).max(moderationRequestSixUserIdMax)
}),zod.object({
  "kind": zod.enum(['sign_out']),
  "user_id": zod.int().min(moderationRequestSevenUserIdMin).max(moderationRequestSevenUserIdMax)
})]);

export type ModerationRequest = zod.input<typeof ModerationRequest>;
export type ModerationRequestOutput = zod.output<typeof ModerationRequest>;

export const moderationResultUserIdMin = 0;
export const moderationResultUserIdMax = 4294967295;



export const ModerationResult = zod.object({
  "message": zod.string(),
  "operation": zod.string(),
  "request_id": zod.string(),
  "until": zod.iso.datetime({"offset":true}).nullish(),
  "user_id": zod.int().min(moderationResultUserIdMin).max(moderationResultUserIdMax)
});

export type ModerationResult = zod.input<typeof ModerationResult>;
export type ModerationResultOutput = zod.output<typeof ModerationResult>;

export const ModerationResponse = zod.object({
  "disconnect": zod.union([EffectState.describe('Null: no live disconnect was needed. Pending/interrupted is an uncertain outcome, never an instruction to repeat it.'),zod.null()]).optional(),
  "result": ModerationResult
});

export type ModerationResponse = zod.input<typeof ModerationResponse>;
export type ModerationResponseOutput = zod.output<typeof ModerationResponse>;

export const onlinePlayerPlayerIdMin = 0;
export const onlinePlayerPlayerIdMax = 4294967295;

export const onlinePlayerUserIdMin = 0;
export const onlinePlayerUserIdMax = 4294967295;



export const OnlinePlayer = zod.object({
  "duelling": zod.boolean(),
  "name": zod.string(),
  "player_id": zod.int().min(onlinePlayerPlayerIdMin).max(onlinePlayerPlayerIdMax),
  "user_id": zod.int().min(onlinePlayerUserIdMin).max(onlinePlayerUserIdMax).nullish()
});

export type OnlinePlayer = zod.input<typeof OnlinePlayer>;
export type OnlinePlayerOutput = zod.output<typeof OnlinePlayer>;

export const RecoveryCodes = zod.object({
  "recovery_codes": zod.array(zod.string())
});

export type RecoveryCodes = zod.input<typeof RecoveryCodes>;
export type RecoveryCodesOutput = zod.output<typeof RecoveryCodes>;

export const regulationChoiceIdMin = 0;



export const RegulationChoice = zod.object({
  "id": zod.int().min(regulationChoiceIdMin),
  "name": zod.string()
});

export type RegulationChoice = zod.input<typeof RegulationChoice>;
export type RegulationChoiceOutput = zod.output<typeof RegulationChoice>;

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

export const reportListOpenMin = 0;
export const reportListOpenMax = 4294967295;



export const ReportList = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "open": zod.int().min(reportListOpenMin).max(reportListOpenMax).describe('Open reports in all, whatever the list shows.'),
  "reports": zod.array(ReportView)
});

export type ReportList = zod.input<typeof ReportList>;
export type ReportListOutput = zod.output<typeof ReportList>;

export const serverOverviewAccountsMin = 0;
export const serverOverviewAccountsMax = 4294967295;

export const serverOverviewDuelsMin = 0;
export const serverOverviewDuelsMax = 4294967295;

export const serverOverviewEnginesMin = 0;
export const serverOverviewEnginesMax = 4294967295;

export const serverOverviewOnlineMin = 0;
export const serverOverviewOnlineMax = 4294967295;

export const serverOverviewPlayersMin = 0;
export const serverOverviewPlayersMax = 4294967295;

export const serverOverviewUptimeSecondsMin = 0;



export const ServerOverview = zod.object({
  "accounts": zod.int().min(serverOverviewAccountsMin).max(serverOverviewAccountsMax),
  "backup": BackupHealth,
  "can_stop": zod.boolean(),
  "checked_at": zod.iso.datetime({"offset":true}),
  "duels": zod.int().min(serverOverviewDuelsMin).max(serverOverviewDuelsMax),
  "engine_problem": zod.string().nullish(),
  "engines": zod.int().min(serverOverviewEnginesMin).max(serverOverviewEnginesMax),
  "memory_bytes": zod.string().nullish().describe('Decimal bytes, preserving precision in JavaScript. Missing on unsupported operating systems.'),
  "online": zod.int().min(serverOverviewOnlineMin).max(serverOverviewOnlineMax),
  "online_players": zod.array(OnlinePlayer),
  "persistence_healthy": zod.boolean(),
  "players": zod.int().min(serverOverviewPlayersMin).max(serverOverviewPlayersMax),
  "run_id": zod.string(),
  "uptime_seconds": zod.int().min(serverOverviewUptimeSecondsMin).nullish(),
  "version": zod.string()
});

export type ServerOverview = zod.input<typeof ServerOverview>;
export type ServerOverviewOutput = zod.output<typeof ServerOverview>;

export const setBanlistRequestBanlistMin = 0;
export const setBanlistRequestBanlistMax = 4294967295;

export const setBanlistRequestUserIdMin = 0;
export const setBanlistRequestUserIdMax = 4294967295;



export const SetBanlistRequest = zod.object({
  "banlist": zod.int().min(setBanlistRequestBanlistMin).max(setBanlistRequestBanlistMax).nullish().describe('A banlist\'s id from the choices; null or omitted goes back to the server\'s.'),
  "user_id": zod.int().min(setBanlistRequestUserIdMin).max(setBanlistRequestUserIdMax)
});

export type SetBanlistRequest = zod.input<typeof SetBanlistRequest>;
export type SetBanlistRequestOutput = zod.output<typeof SetBanlistRequest>;

export const settingKindOneMaxLengthMin = 0;

export const settingKindTwoMaxMin = 0;

export const settingKindTwoMinMin = 0;



export const SettingKind = zod.union([zod.object({
  "kind": zod.enum(['text']),
  "max_length": zod.int().min(settingKindOneMaxLengthMin)
}),zod.object({
  "kind": zod.enum(['number']),
  "max": zod.int().min(settingKindTwoMaxMin),
  "min": zod.int().min(settingKindTwoMinMin)
}),zod.object({
  "choices": zod.array(zod.string()),
  "kind": zod.enum(['choice'])
}),zod.object({
  "kind": zod.enum(['version'])
}),zod.object({
  "choices": zod.array(RegulationChoice),
  "kind": zod.enum(['regulation'])
}),zod.object({
  "kind": zod.enum(['news'])
})]);

export type SettingKind = zod.input<typeof SettingKind>;
export type SettingKindOutput = zod.output<typeof SettingKind>;

export const SettingRow = zod.object({
  "applies": zod.string(),
  "default_value": zod.string(),
  "help": zod.string(),
  "input": SettingKind,
  "name": zod.string(),
  "overridden": zod.boolean(),
  "value": zod.string()
});

export type SettingRow = zod.input<typeof SettingRow>;
export type SettingRowOutput = zod.output<typeof SettingRow>;

export const SettingsList = zod.object({
  "checked_at": zod.iso.datetime({"offset":true}),
  "maintenance_active": zod.boolean(),
  "maintenance_at": zod.iso.datetime({"offset":true}).nullish(),
  "settings": zod.array(SettingRow)
});

export type SettingsList = zod.input<typeof SettingsList>;
export type SettingsListOutput = zod.output<typeof SettingsList>;

export const settingsRequestThreeMinutesMin = 0;
export const settingsRequestThreeMinutesMax = 4294967295;



export const SettingsRequest = zod.union([zod.object({
  "kind": zod.enum(['set']),
  "name": zod.string(),
  "value": zod.string()
}),zod.object({
  "kind": zod.enum(['reset']),
  "name": zod.string()
}),zod.object({
  "kind": zod.enum(['schedule_maintenance']),
  "message": zod.string().nullish(),
  "minutes": zod.int().min(settingsRequestThreeMinutesMin).max(settingsRequestThreeMinutesMax)
}),zod.object({
  "kind": zod.enum(['cancel_maintenance'])
})]);

export type SettingsRequest = zod.input<typeof SettingsRequest>;
export type SettingsRequestOutput = zod.output<typeof SettingsRequest>;

export const SettingsResult = zod.object({
  "maintenance_at": zod.iso.datetime({"offset":true}).nullish().describe('Committed deadline, not a live status. A retry returns this original deadline even after later changes.'),
  "message": zod.string(),
  "request_id": zod.string()
});

export type SettingsResult = zod.input<typeof SettingsResult>;
export type SettingsResultOutput = zod.output<typeof SettingsResult>;

export const signInCodeRequestChallengeMin = 64;
export const signInCodeRequestChallengeMax = 64;

export const signInCodeRequestCodeMin = 6;
export const signInCodeRequestCodeMax = 32;



export const SignInCodeRequest = zod.object({
  "challenge": zod.string().min(signInCodeRequestChallengeMin).max(signInCodeRequestChallengeMax),
  "code": zod.string().min(signInCodeRequestCodeMin).max(signInCodeRequestCodeMax).describe('An authenticator code (six digits) or an unused recovery code.')
});

export type SignInCodeRequest = zod.input<typeof SignInCodeRequest>;
export type SignInCodeRequestOutput = zod.output<typeof SignInCodeRequest>;

export const SignInOutcome = zod.object({
  "challenge": zod.string().nullish().describe('Present with `code_required`: send it back with the code to `/sign-in/code` within five minutes.'),
  "code_required": zod.boolean(),
  "session": zod.union([StaffSession,zod.null()]).optional()
}).describe('The result of the password step: a code is still owed (`challenge`), or a session was opened (see its `limited`).');

export type SignInOutcome = zod.input<typeof SignInOutcome>;
export type SignInOutcomeOutput = zod.output<typeof SignInOutcome>;

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

export const stopRequestRunIdMin = 64;
export const stopRequestRunIdMax = 64;


export const stopRequestRunIdRegExp = new RegExp('^[a-f0-9]{64}$');


export const StopRequest = zod.object({
  "run_id": zod.string().min(stopRequestRunIdMin).max(stopRequestRunIdMax).regex(stopRequestRunIdRegExp)
});

export type StopRequest = zod.input<typeof StopRequest>;
export type StopRequestOutput = zod.output<typeof StopRequest>;

export const StopResult = zod.object({
  "message": zod.string(),
  "request_id": zod.string(),
  "run_id": zod.string()
});

export type StopResult = zod.input<typeof StopResult>;
export type StopResultOutput = zod.output<typeof StopResult>;

export const StopResponse = zod.object({
  "result": StopResult,
  "stop": EffectState
});

export type StopResponse = zod.input<typeof StopResponse>;
export type StopResponseOutput = zod.output<typeof StopResponse>;
export const ListAccounts200Response = AccountList

export const ListAccounts400Response = ApiError

export const ListAccounts401Response = ApiError

export const ListAccounts403Response = ApiError

export const ListAccounts405Response = ApiError

export const ListAccounts413Response = ApiError

export const ListAccounts415Response = ApiError

export const ListAccounts500Response = ApiError

export const ListAccounts503Response = ApiError


export const setAccountBanlistHeaderIdempotencyKeyMin = 16;
export const setAccountBanlistHeaderIdempotencyKeyMax = 128;


export const setAccountBanlistHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const SetAccountBanlistHeader = zod.object({
  "Idempotency-Key": zod.string().min(setAccountBanlistHeaderIdempotencyKeyMin).max(setAccountBanlistHeaderIdempotencyKeyMax).regex(setAccountBanlistHeaderIdempotencyKeyRegExp).describe('One ID per change; the change is the same when repeated.')
})

export const SetAccountBanlistBody = SetBanlistRequest

export const SetAccountBanlist200Response = AccountBanlist

export const SetAccountBanlist400Response = ApiError

export const SetAccountBanlist401Response = ApiError

export const SetAccountBanlist403Response = ApiError

export const SetAccountBanlist404Response = ApiError

export const SetAccountBanlist405Response = ApiError

export const SetAccountBanlist409Response = ApiError

export const SetAccountBanlist413Response = ApiError

export const SetAccountBanlist415Response = ApiError

export const SetAccountBanlist500Response = ApiError

export const SetAccountBanlist503Response = ApiError


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


export const deleteAccountHeaderIdempotencyKeyMin = 16;
export const deleteAccountHeaderIdempotencyKeyMax = 128;


export const deleteAccountHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const DeleteAccountHeader = zod.object({
  "Idempotency-Key": zod.string().min(deleteAccountHeaderIdempotencyKeyMin).max(deleteAccountHeaderIdempotencyKeyMax).regex(deleteAccountHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A retry observes the original saved outcome.')
})

export const DeleteAccountBody = DeleteAccountRequest

export const DeleteAccount200Response = DeleteAccountResponse

export const DeleteAccount400Response = ApiError

export const DeleteAccount401Response = ApiError

export const DeleteAccount403Response = ApiError

export const DeleteAccount404Response = ApiError

export const DeleteAccount405Response = ApiError

export const DeleteAccount409Response = ApiError

export const DeleteAccount413Response = ApiError

export const DeleteAccount415Response = ApiError

export const DeleteAccount500Response = ApiError

export const DeleteAccount503Response = ApiError


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


export const getAuditHistoryQueryLimitMax = 200;

export const getAuditHistoryQueryBeforeMax = 19;


export const getAuditHistoryQueryBeforeRegExp = new RegExp('^[1-9][0-9]*$');
export const getAuditHistoryQueryQMax = 80;



export const GetAuditHistoryQueryParams = zod.object({
  "limit": zod.int().min(1).max(getAuditHistoryQueryLimitMax).optional(),
  "before": zod.string().min(1).max(getAuditHistoryQueryBeforeMax).regex(getAuditHistoryQueryBeforeRegExp).optional().describe('Decimal ID returned by next_before; never a JS number'),
  "q": zod.string().max(getAuditHistoryQueryQMax).optional()
})

export const GetAuditHistory200Response = AuditPage

export const GetAuditHistory400Response = ApiError

export const GetAuditHistory401Response = ApiError

export const GetAuditHistory403Response = ApiError

export const GetAuditHistory404Response = ApiError

export const GetAuditHistory405Response = ApiError

export const GetAuditHistory413Response = ApiError

export const GetAuditHistory415Response = ApiError

export const GetAuditHistory500Response = ApiError

export const GetAuditHistory503Response = ApiError


export const ListBackups200Response = BackupList

export const ListBackups400Response = ApiError

export const ListBackups401Response = ApiError

export const ListBackups403Response = ApiError

export const ListBackups404Response = ApiError

export const ListBackups405Response = ApiError

export const ListBackups413Response = ApiError

export const ListBackups415Response = ApiError

export const ListBackups500Response = ApiError

export const ListBackups503Response = ApiError


export const createBackupHeaderIdempotencyKeyMin = 16;
export const createBackupHeaderIdempotencyKeyMax = 128;


export const createBackupHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const CreateBackupHeader = zod.object({
  "Idempotency-Key": zod.string().min(createBackupHeaderIdempotencyKeyMin).max(createBackupHeaderIdempotencyKeyMax).regex(createBackupHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A retry observes the original saved outcome.')
})

export const CreateBackupBody = CreateBackupRequest

export const CreateBackup200Response = CreateBackupResponse

export const CreateBackup400Response = ApiError

export const CreateBackup401Response = ApiError

export const CreateBackup403Response = ApiError

export const CreateBackup404Response = ApiError

export const CreateBackup405Response = ApiError

export const CreateBackup409Response = ApiError

export const CreateBackup413Response = ApiError

export const CreateBackup415Response = ApiError

export const CreateBackup500Response = ApiError

export const CreateBackup503Response = ApiError


export const downloadBackupHeaderIdempotencyKeyMin = 16;
export const downloadBackupHeaderIdempotencyKeyMax = 128;


export const downloadBackupHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const DownloadBackupHeader = zod.object({
  "Idempotency-Key": zod.string().min(downloadBackupHeaderIdempotencyKeyMin).max(downloadBackupHeaderIdempotencyKeyMax).regex(downloadBackupHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A retry observes the original saved outcome.')
})

export const DownloadBackupBody = DownloadBackupRequest

export const DownloadBackup200Response = zod.unknown()

export const DownloadBackup400Response = ApiError

export const DownloadBackup401Response = ApiError

export const DownloadBackup403Response = ApiError

export const DownloadBackup404Response = ApiError

export const DownloadBackup405Response = ApiError

export const DownloadBackup409Response = ApiError

export const DownloadBackup413Response = ApiError

export const DownloadBackup415Response = ApiError

export const DownloadBackup500Response = ApiError

export const DownloadBackup503Response = ApiError


export const ListBanlists200Response = BanlistChoices

export const ListBanlists400Response = ApiError

export const ListBanlists401Response = ApiError

export const ListBanlists403Response = ApiError

export const ListBanlists404Response = ApiError

export const ListBanlists405Response = ApiError

export const ListBanlists409Response = ApiError

export const ListBanlists413Response = ApiError

export const ListBanlists415Response = ApiError

export const ListBanlists500Response = ApiError

export const ListBanlists503Response = ApiError


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


export const getCardImagePathRowMin = 0;



export const GetCardImageParams = zod.object({
  "row": zod.int().min(getCardImagePathRowMin),
  "size": zod.enum(['mini', 'full'])
})

export const GetCardImageQueryParams = zod.object({
  "v": zod.string().describe('Archive revision from the gift catalogue')
})

export const GetCardImageHeader = zod.object({
  "If-None-Match": zod.string().optional().describe('Previously received ETag')
})

export const GetCardImage200Response = zod.unknown()

export const GetCardImage304Response = zod.unknown()

export const GetCardImage400Response = ApiError

export const GetCardImage401Response = ApiError

export const GetCardImage403Response = ApiError

export const GetCardImage404Response = ApiError

export const GetCardImage405Response = ApiError

export const GetCardImage413Response = ApiError

export const GetCardImage415Response = ApiError

export const GetCardImage500Response = ApiError


export const ListCountries200Response = CountryList

export const ListCountries400Response = ApiError

export const ListCountries401Response = ApiError

export const ListCountries403Response = ApiError

export const ListCountries405Response = ApiError

export const ListCountries413Response = ApiError

export const ListCountries415Response = ApiError

export const ListCountries500Response = ApiError

export const ListCountries503Response = ApiError


export const givePlayerGiftHeaderIdempotencyKeyMin = 16;
export const givePlayerGiftHeaderIdempotencyKeyMax = 128;


export const givePlayerGiftHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const GivePlayerGiftHeader = zod.object({
  "Idempotency-Key": zod.string().min(givePlayerGiftHeaderIdempotencyKeyMin).max(givePlayerGiftHeaderIdempotencyKeyMax).regex(givePlayerGiftHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. Retries never give duplicate cards or repeat mail or game notifications.')
})

export const GivePlayerGiftBody = GiftRequest

export const GivePlayerGift200Response = GiftResult

export const GivePlayerGift400Response = ApiError

export const GivePlayerGift401Response = ApiError

export const GivePlayerGift403Response = ApiError

export const GivePlayerGift404Response = ApiError

export const GivePlayerGift405Response = ApiError

export const GivePlayerGift409Response = ApiError

export const GivePlayerGift413Response = ApiError

export const GivePlayerGift415Response = ApiError

export const GivePlayerGift500Response = ApiError

export const GivePlayerGift503Response = ApiError


export const listGiftChoicesQueryQMax = 80;



export const ListGiftChoicesQueryParams = zod.object({
  "q": zod.string().max(listGiftChoicesQueryQMax).optional().describe('Search game metadata by name or ID; at most 50 results per kind')
})

export const ListGiftChoices200Response = GiftChoices

export const ListGiftChoices400Response = ApiError

export const ListGiftChoices401Response = ApiError

export const ListGiftChoices403Response = ApiError

export const ListGiftChoices405Response = ApiError

export const ListGiftChoices413Response = ApiError

export const ListGiftChoices415Response = ApiError

export const ListGiftChoices500Response = ApiError

export const ListGiftChoices503Response = ApiError


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


export const getServerLogsQueryLimitMax = 500;



export const GetServerLogsQueryParams = zod.object({
  "limit": zod.int().min(1).max(getServerLogsQueryLimitMax).optional(),
  "problems": zod.boolean().optional().describe('Only warnings and errors')
})

export const GetServerLogs200Response = LogPage

export const GetServerLogs400Response = ApiError

export const GetServerLogs401Response = ApiError

export const GetServerLogs403Response = ApiError

export const GetServerLogs404Response = ApiError

export const GetServerLogs405Response = ApiError

export const GetServerLogs413Response = ApiError

export const GetServerLogs415Response = ApiError

export const GetServerLogs500Response = ApiError

export const GetServerLogs503Response = ApiError


export const startMfaEnrollmentHeaderIdempotencyKeyMin = 16;
export const startMfaEnrollmentHeaderIdempotencyKeyMax = 128;


export const startMfaEnrollmentHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const StartMfaEnrollmentHeader = zod.object({
  "Idempotency-Key": zod.string().min(startMfaEnrollmentHeaderIdempotencyKeyMin).max(startMfaEnrollmentHeaderIdempotencyKeyMax).regex(startMfaEnrollmentHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const StartMfaEnrollment200Response = MfaEnrollmentStart

export const StartMfaEnrollment400Response = ApiError

export const StartMfaEnrollment401Response = ApiError

export const StartMfaEnrollment403Response = ApiError

export const StartMfaEnrollment405Response = ApiError

export const StartMfaEnrollment413Response = ApiError

export const StartMfaEnrollment415Response = ApiError

export const StartMfaEnrollment429Response = ApiError

export const StartMfaEnrollment500Response = ApiError


export const confirmMfaEnrollmentHeaderIdempotencyKeyMin = 16;
export const confirmMfaEnrollmentHeaderIdempotencyKeyMax = 128;


export const confirmMfaEnrollmentHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const ConfirmMfaEnrollmentHeader = zod.object({
  "Idempotency-Key": zod.string().min(confirmMfaEnrollmentHeaderIdempotencyKeyMin).max(confirmMfaEnrollmentHeaderIdempotencyKeyMax).regex(confirmMfaEnrollmentHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const ConfirmMfaEnrollmentBody = CodeRequest

export const ConfirmMfaEnrollment200Response = MfaEnrolled

export const ConfirmMfaEnrollment400Response = ApiError

export const ConfirmMfaEnrollment401Response = ApiError

export const ConfirmMfaEnrollment403Response = ApiError

export const ConfirmMfaEnrollment405Response = ApiError

export const ConfirmMfaEnrollment413Response = ApiError

export const ConfirmMfaEnrollment415Response = ApiError

export const ConfirmMfaEnrollment429Response = ApiError

export const ConfirmMfaEnrollment500Response = ApiError


export const regenerateRecoveryCodesHeaderIdempotencyKeyMin = 16;
export const regenerateRecoveryCodesHeaderIdempotencyKeyMax = 128;


export const regenerateRecoveryCodesHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const RegenerateRecoveryCodesHeader = zod.object({
  "Idempotency-Key": zod.string().min(regenerateRecoveryCodesHeaderIdempotencyKeyMin).max(regenerateRecoveryCodesHeaderIdempotencyKeyMax).regex(regenerateRecoveryCodesHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const RegenerateRecoveryCodesBody = EmptyRequest

export const RegenerateRecoveryCodes200Response = RecoveryCodes

export const RegenerateRecoveryCodes400Response = ApiError

export const RegenerateRecoveryCodes401Response = ApiError

export const RegenerateRecoveryCodes403Response = ApiError

export const RegenerateRecoveryCodes405Response = ApiError

export const RegenerateRecoveryCodes413Response = ApiError

export const RegenerateRecoveryCodes415Response = ApiError

export const RegenerateRecoveryCodes429Response = ApiError

export const RegenerateRecoveryCodes500Response = ApiError


export const confirmMfaStepUpHeaderIdempotencyKeyMin = 16;
export const confirmMfaStepUpHeaderIdempotencyKeyMax = 128;


export const confirmMfaStepUpHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const ConfirmMfaStepUpHeader = zod.object({
  "Idempotency-Key": zod.string().min(confirmMfaStepUpHeaderIdempotencyKeyMin).max(confirmMfaStepUpHeaderIdempotencyKeyMax).regex(confirmMfaStepUpHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const ConfirmMfaStepUpBody = CodeRequest

export const ConfirmMfaStepUp200Response = StaffSession

export const ConfirmMfaStepUp400Response = ApiError

export const ConfirmMfaStepUp401Response = ApiError

export const ConfirmMfaStepUp403Response = ApiError

export const ConfirmMfaStepUp405Response = ApiError

export const ConfirmMfaStepUp413Response = ApiError

export const ConfirmMfaStepUp415Response = ApiError

export const ConfirmMfaStepUp429Response = ApiError

export const ConfirmMfaStepUp500Response = ApiError


export const moderateAccountHeaderIdempotencyKeyMin = 16;
export const moderateAccountHeaderIdempotencyKeyMax = 128;


export const moderateAccountHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const ModerateAccountHeader = zod.object({
  "Idempotency-Key": zod.string().min(moderateAccountHeaderIdempotencyKeyMin).max(moderateAccountHeaderIdempotencyKeyMax).regex(moderateAccountHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A retry observes the original saved outcome.')
})

export const ModerateAccountBody = ModerationRequest

export const ModerateAccount200Response = ModerationResponse

export const ModerateAccount400Response = ApiError

export const ModerateAccount401Response = ApiError

export const ModerateAccount403Response = ApiError

export const ModerateAccount404Response = ApiError

export const ModerateAccount405Response = ApiError

export const ModerateAccount409Response = ApiError

export const ModerateAccount413Response = ApiError

export const ModerateAccount415Response = ApiError

export const ModerateAccount500Response = ApiError

export const ModerateAccount503Response = ApiError


export const listReportsQueryLimitMax = 500;



export const ListReportsQueryParams = zod.object({
  "status": zod.enum(['open', 'handled', 'dismissed']).optional().describe('open, handled or dismissed; all when omitted.'),
  "limit": zod.int().min(1).max(listReportsQueryLimitMax).optional()
})

export const ListReports200Response = ReportList

export const ListReports400Response = ApiError

export const ListReports401Response = ApiError

export const ListReports403Response = ApiError

export const ListReports404Response = ApiError

export const ListReports405Response = ApiError

export const ListReports409Response = ApiError

export const ListReports413Response = ApiError

export const ListReports415Response = ApiError

export const ListReports500Response = ApiError

export const ListReports503Response = ApiError


export const handleReportHeaderIdempotencyKeyMin = 16;
export const handleReportHeaderIdempotencyKeyMax = 128;


export const handleReportHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const HandleReportHeader = zod.object({
  "Idempotency-Key": zod.string().min(handleReportHeaderIdempotencyKeyMin).max(handleReportHeaderIdempotencyKeyMax).regex(handleReportHeaderIdempotencyKeyRegExp).describe('One ID per change; marking the same report the same way again answers as done.')
})

export const HandleReportBody = HandleReportRequest

export const HandleReport200Response = HandleReportResult

export const HandleReport400Response = ApiError

export const HandleReport401Response = ApiError

export const HandleReport403Response = ApiError

export const HandleReport404Response = ApiError

export const HandleReport405Response = ApiError

export const HandleReport409Response = ApiError

export const HandleReport413Response = ApiError

export const HandleReport415Response = ApiError

export const HandleReport500Response = ApiError

export const HandleReport503Response = ApiError


export const GetServerStatus200Response = ServerOverview

export const GetServerStatus400Response = ApiError

export const GetServerStatus401Response = ApiError

export const GetServerStatus403Response = ApiError

export const GetServerStatus405Response = ApiError

export const GetServerStatus413Response = ApiError

export const GetServerStatus415Response = ApiError

export const GetServerStatus500Response = ApiError


export const stopServerHeaderIdempotencyKeyMin = 16;
export const stopServerHeaderIdempotencyKeyMax = 128;


export const stopServerHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const StopServerHeader = zod.object({
  "Idempotency-Key": zod.string().min(stopServerHeaderIdempotencyKeyMin).max(stopServerHeaderIdempotencyKeyMax).regex(stopServerHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. A retry observes the original saved outcome.')
})

export const StopServerBody = StopRequest

export const StopServer200Response = StopResponse

export const StopServer400Response = ApiError

export const StopServer401Response = ApiError

export const StopServer403Response = ApiError

export const StopServer404Response = ApiError

export const StopServer405Response = ApiError

export const StopServer409Response = ApiError

export const StopServer413Response = ApiError

export const StopServer415Response = ApiError

export const StopServer500Response = ApiError

export const StopServer503Response = ApiError


export const GetStaffSession200Response = StaffSession

export const GetStaffSession400Response = ApiError

export const GetStaffSession401Response = ApiError

export const GetStaffSession403Response = ApiError

export const GetStaffSession405Response = ApiError

export const GetStaffSession413Response = ApiError

export const GetStaffSession415Response = ApiError

export const GetStaffSession500Response = ApiError


export const ListSettings200Response = SettingsList

export const ListSettings400Response = ApiError

export const ListSettings401Response = ApiError

export const ListSettings403Response = ApiError

export const ListSettings405Response = ApiError

export const ListSettings413Response = ApiError

export const ListSettings415Response = ApiError

export const ListSettings500Response = ApiError

export const ListSettings503Response = ApiError


export const changeSettingsHeaderIdempotencyKeyMin = 16;
export const changeSettingsHeaderIdempotencyKeyMax = 128;


export const changeSettingsHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const ChangeSettingsHeader = zod.object({
  "Idempotency-Key": zod.string().min(changeSettingsHeaderIdempotencyKeyMin).max(changeSettingsHeaderIdempotencyKeyMax).regex(changeSettingsHeaderIdempotencyKeyRegExp).describe('Retain this ID and input on retry. Retries return the original result without restoring old settings or moving a countdown.')
})

export const ChangeSettingsBody = SettingsRequest

export const ChangeSettings200Response = SettingsResult

export const ChangeSettings400Response = ApiError

export const ChangeSettings401Response = ApiError

export const ChangeSettings403Response = ApiError

export const ChangeSettings404Response = ApiError

export const ChangeSettings405Response = ApiError

export const ChangeSettings409Response = ApiError

export const ChangeSettings413Response = ApiError

export const ChangeSettings415Response = ApiError

export const ChangeSettings500Response = ApiError

export const ChangeSettings503Response = ApiError


export const signInStaffHeaderIdempotencyKeyMin = 16;
export const signInStaffHeaderIdempotencyKeyMax = 128;


export const signInStaffHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const SignInStaffHeader = zod.object({
  "Idempotency-Key": zod.string().min(signInStaffHeaderIdempotencyKeyMin).max(signInStaffHeaderIdempotencyKeyMax).regex(signInStaffHeaderIdempotencyKeyRegExp).describe('One ID per sign-in attempt. Authentication does not create a durable operation receipt.')
})

export const SignInStaffBody = SignInRequest

export const SignInStaff200Response = SignInOutcome

export const SignInStaff400Response = ApiError

export const SignInStaff401Response = ApiError

export const SignInStaff403Response = ApiError

export const SignInStaff405Response = ApiError

export const SignInStaff413Response = ApiError

export const SignInStaff415Response = ApiError

export const SignInStaff429Response = ApiError

export const SignInStaff500Response = ApiError


export const verifyStaffCodeHeaderIdempotencyKeyMin = 16;
export const verifyStaffCodeHeaderIdempotencyKeyMax = 128;


export const verifyStaffCodeHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const VerifyStaffCodeHeader = zod.object({
  "Idempotency-Key": zod.string().min(verifyStaffCodeHeaderIdempotencyKeyMin).max(verifyStaffCodeHeaderIdempotencyKeyMax).regex(verifyStaffCodeHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const VerifyStaffCodeBody = SignInCodeRequest

export const VerifyStaffCode200Response = StaffSession

export const VerifyStaffCode400Response = ApiError

export const VerifyStaffCode401Response = ApiError

export const VerifyStaffCode403Response = ApiError

export const VerifyStaffCode405Response = ApiError

export const VerifyStaffCode413Response = ApiError

export const VerifyStaffCode415Response = ApiError

export const VerifyStaffCode429Response = ApiError

export const VerifyStaffCode500Response = ApiError


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


export const signOutEverywhereHeaderIdempotencyKeyMin = 16;
export const signOutEverywhereHeaderIdempotencyKeyMax = 128;


export const signOutEverywhereHeaderIdempotencyKeyRegExp = new RegExp('^[A-Za-z0-9_-]+$');


export const SignOutEverywhereHeader = zod.object({
  "Idempotency-Key": zod.string().min(signOutEverywhereHeaderIdempotencyKeyMin).max(signOutEverywhereHeaderIdempotencyKeyMax).regex(signOutEverywhereHeaderIdempotencyKeyRegExp).describe('One ID per attempt. Authentication does not create a durable operation receipt.')
})

export const SignOutEverywhereBody = EmptyRequest

export const SignOutEverywhere200Response = SignedOut

export const SignOutEverywhere400Response = ApiError

export const SignOutEverywhere401Response = ApiError

export const SignOutEverywhere403Response = ApiError

export const SignOutEverywhere405Response = ApiError

export const SignOutEverywhere413Response = ApiError

export const SignOutEverywhere415Response = ApiError

export const SignOutEverywhere429Response = ApiError

export const SignOutEverywhere500Response = ApiError
