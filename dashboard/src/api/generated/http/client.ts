/**
 * Generated from Rust endpoints by Orval. DO NOT EDIT.
 * Regenerate: npm run api:generate
 */
import {
  useMutation,
  useQuery
} from '@tanstack/react-query';
import type {
  DataTag,
  DefinedInitialDataOptions,
  DefinedUseQueryResult,
  MutationFunction,
  QueryClient,
  QueryFunction,
  QueryKey,
  UndefinedInitialDataOptions,
  UseMutationOptions,
  UseMutationResult,
  UseQueryOptions,
  UseQueryResult
} from '@tanstack/react-query';

import type {
  AccountBanlist,
  AccountCodeResponse,
  AccountDetail,
  AccountEditRequest,
  AccountEditResult,
  AccountList,
  ApiError,
  AuditPage,
  BackupList,
  BanlistChoices,
  BpCodeList,
  CancelChampionshipHeaders,
  CancelChampionshipRequest,
  CancelEventHeaders,
  CancelEventRequest,
  CardImageSize,
  ChampionshipList,
  ChampionshipResult,
  ChangeSettingsHeaders,
  CodeRequest,
  ConfirmMfaEnrollmentHeaders,
  ConfirmMfaStepUpHeaders,
  CountryList,
  CreateBackupHeaders,
  CreateBackupRequest,
  CreateBackupResponse,
  CreateBpCodeHeaders,
  CreateBpCodeRequest,
  CreateBpCodeResult,
  CreateChampionshipHeaders,
  CreateChampionshipRequest,
  CreateEventHeaders,
  CreateEventRequest,
  CreateInviteHeaders,
  CreateInviteRequest,
  CreateInviteResult,
  DeleteAccountHeaders,
  DeleteAccountRequest,
  DeleteAccountResponse,
  DownloadBackupHeaders,
  DownloadBackupRequest,
  EditAccountHeaders,
  EditChampionshipHeaders,
  EditChampionshipRequest,
  EditEventHeaders,
  EditEventRequest,
  EmptyRequest,
  EventList,
  EventResult,
  GetAuditHistoryParams,
  GetCardImageHeaders,
  GetCardImageParams,
  GetServerLogsParams,
  GiftChoices,
  GiftRequest,
  GiftResult,
  GiveBpByIdRequest,
  GiveBpHeaders,
  GiveBpResult,
  GivePlayerGiftHeaders,
  HandleReportHeaders,
  HandleReportRequest,
  HandleReportResult,
  InviteList,
  IssueAccountCodeHeaders,
  IssueAccountCodeRequest,
  KeyAccountRequest,
  KeyAccountResult,
  ListGiftChoicesParams,
  ListProblemReportsParams,
  ListReportsParams,
  LogPage,
  ManageAccountKeyHeaders,
  MfaEnrolled,
  MfaEnrollmentStart,
  ModerateAccountHeaders,
  ModerationRequest,
  ModerationResponse,
  ProblemReport,
  ProblemReportList,
  RecoveryCodes,
  RegenerateRecoveryCodesHeaders,
  RemoveBpCodeHeaders,
  RemoveBpCodeRequest,
  RemoveBpCodeResult,
  ReportList,
  ServerOverview,
  SetAccountBanlistHeaders,
  SetBanlistRequest,
  SettingsList,
  SettingsRequest,
  SettingsResult,
  SignInCodeRequest,
  SignInOutcome,
  SignInRequest,
  SignInStaffHeaders,
  SignOutEverywhereHeaders,
  SignOutStaffHeaders,
  SignedOut,
  StaffSession,
  StartMfaEnrollmentHeaders,
  StopRequest,
  StopResponse,
  StopServerHeaders,
  VerifyStaffCodeHeaders
} from './models';

import { apiFetch } from '../../transport';
import type { ErrorType } from '../../transport';
type SecondParameter<T extends (...args: never) => unknown> = Parameters<T>[1];


const getHeaders = (...sources: (RequestInit['headers'] | Record<string, string | undefined>)[]) => {
  const seen = new Set<string>();
  const layers: [string, string | readonly string[]][][] = [];
  for (const source of sources.reverse()) {
    if (!source) continue;
    const entries = source instanceof Headers
      ? source.entries()
      : Symbol.iterator in source
        ? Array.from(source as Iterable<Iterable<string>>, (entry) => Array.from(entry) as [string, string])
        : Object.entries<string | readonly string[] | undefined>(source);
    const layer: [string, string | readonly string[]][] = [];
    for (const [name, value] of entries) {
      if (value !== undefined && !seen.has(name.toLowerCase())) layer.push([name, value]);
    }
    for (const [name] of layer) seen.add(name.toLowerCase());
    layers.push(layer);
  }
  return Object.fromEntries(layers.reverse().flat()) as Extract<RequestInit['headers'], Record<string, unknown>>;
};


const withQueryKey = <T extends object, K>(query: T, queryKey: K): T & { queryKey: K } => {
  const result = { queryKey } as T & { queryKey: K };
  for (const key of Object.keys(query)) {
    // The explicit queryKey always wins, matching the previous
    // `{ ...query, queryKey }` spread where it was set last.
    if (key === 'queryKey') continue;
    Object.defineProperty(result, key, {
      enumerable: true,
      configurable: true,
      get: () => (query as Record<string, unknown>)[key],
    });
  }
  return result;
};

export const getListAccountsUrl = () => {




  return `/panel/api/v1/accounts`
}

export const listAccounts = async ( options?: Parameters<typeof apiFetch>[1]): Promise<AccountList> => {

  return apiFetch<AccountList>(getListAccountsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListAccountsQueryKey = () => {
    return [
    `/panel/api/v1/accounts`
    ] as const;
    }


export const getListAccountsQueryOptions = <TData = Awaited<ReturnType<typeof listAccounts>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListAccountsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listAccounts>>> = ({ signal }) => listAccounts({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListAccountsQueryResult = NonNullable<Awaited<ReturnType<typeof listAccounts>>>
export type ListAccountsQueryError = ErrorType<ApiError>


export function useListAccounts<TData = Awaited<ReturnType<typeof listAccounts>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listAccounts>>,
          TError,
          Awaited<ReturnType<typeof listAccounts>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListAccounts<TData = Awaited<ReturnType<typeof listAccounts>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listAccounts>>,
          TError,
          Awaited<ReturnType<typeof listAccounts>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListAccounts<TData = Awaited<ReturnType<typeof listAccounts>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListAccounts<TData = Awaited<ReturnType<typeof listAccounts>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listAccounts>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListAccountsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getSetAccountBanlistUrl = () => {




  return `/panel/api/v1/accounts/banlist`
}

export const setAccountBanlist = async (setBanlistRequest: SetBanlistRequest,
    headers: SetAccountBanlistHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<AccountBanlist> => {

  return apiFetch<AccountBanlist>(getSetAccountBanlistUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(setBanlistRequest)
  }
);}





export const getSetAccountBanlistMutationKey = () => ['setAccountBanlist'] as const;

export const getSetAccountBanlistMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof setAccountBanlist>>, TError,SetAccountBanlistMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof setAccountBanlist>>, TError,SetAccountBanlistMutationVariables, TContext> => {

const mutationKey = getSetAccountBanlistMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof setAccountBanlist>>, SetAccountBanlistMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  setAccountBanlist(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type SetAccountBanlistMutationResult = NonNullable<Awaited<ReturnType<typeof setAccountBanlist>>>
    export type SetAccountBanlistMutationBody = SetBanlistRequest
    export type SetAccountBanlistMutationError = ErrorType<ApiError>
    export type SetAccountBanlistMutationVariables = {data: SetBanlistRequest;headers: SetAccountBanlistHeaders}

    export const useSetAccountBanlist = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof setAccountBanlist>>, TError,SetAccountBanlistMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof setAccountBanlist>>,
        TError,
        SetAccountBanlistMutationVariables,
        TContext
      > => {
      return useMutation(getSetAccountBanlistMutationOptions(options), queryClient);
    }

export const getIssueAccountCodeUrl = () => {




  return `/panel/api/v1/accounts/credentials`
}

export const issueAccountCode = async (issueAccountCodeRequest: IssueAccountCodeRequest,
    headers: IssueAccountCodeHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<AccountCodeResponse> => {

  return apiFetch<AccountCodeResponse>(getIssueAccountCodeUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(issueAccountCodeRequest)
  }
);}





export const getIssueAccountCodeMutationKey = () => ['issueAccountCode'] as const;

export const getIssueAccountCodeMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof issueAccountCode>>, TError,IssueAccountCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof issueAccountCode>>, TError,IssueAccountCodeMutationVariables, TContext> => {

const mutationKey = getIssueAccountCodeMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof issueAccountCode>>, IssueAccountCodeMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  issueAccountCode(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type IssueAccountCodeMutationResult = NonNullable<Awaited<ReturnType<typeof issueAccountCode>>>
    export type IssueAccountCodeMutationBody = IssueAccountCodeRequest
    export type IssueAccountCodeMutationError = ErrorType<ApiError>
    export type IssueAccountCodeMutationVariables = {data: IssueAccountCodeRequest;headers: IssueAccountCodeHeaders}

    export const useIssueAccountCode = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof issueAccountCode>>, TError,IssueAccountCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof issueAccountCode>>,
        TError,
        IssueAccountCodeMutationVariables,
        TContext
      > => {
      return useMutation(getIssueAccountCodeMutationOptions(options), queryClient);
    }

export const getDeleteAccountUrl = () => {




  return `/panel/api/v1/accounts/delete`
}

export const deleteAccount = async (deleteAccountRequest: DeleteAccountRequest,
    headers: DeleteAccountHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<DeleteAccountResponse> => {

  return apiFetch<DeleteAccountResponse>(getDeleteAccountUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(deleteAccountRequest)
  }
);}





export const getDeleteAccountMutationKey = () => ['deleteAccount'] as const;

export const getDeleteAccountMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof deleteAccount>>, TError,DeleteAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof deleteAccount>>, TError,DeleteAccountMutationVariables, TContext> => {

const mutationKey = getDeleteAccountMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof deleteAccount>>, DeleteAccountMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  deleteAccount(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type DeleteAccountMutationResult = NonNullable<Awaited<ReturnType<typeof deleteAccount>>>
    export type DeleteAccountMutationBody = DeleteAccountRequest
    export type DeleteAccountMutationError = ErrorType<ApiError>
    export type DeleteAccountMutationVariables = {data: DeleteAccountRequest;headers: DeleteAccountHeaders}

    export const useDeleteAccount = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof deleteAccount>>, TError,DeleteAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof deleteAccount>>,
        TError,
        DeleteAccountMutationVariables,
        TContext
      > => {
      return useMutation(getDeleteAccountMutationOptions(options), queryClient);
    }

export const getEditAccountUrl = () => {




  return `/panel/api/v1/accounts/edit`
}

export const editAccount = async (accountEditRequest: AccountEditRequest,
    headers: EditAccountHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<AccountEditResult> => {

  return apiFetch<AccountEditResult>(getEditAccountUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(accountEditRequest)
  }
);}





export const getEditAccountMutationKey = () => ['editAccount'] as const;

export const getEditAccountMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editAccount>>, TError,EditAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof editAccount>>, TError,EditAccountMutationVariables, TContext> => {

const mutationKey = getEditAccountMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof editAccount>>, EditAccountMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  editAccount(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type EditAccountMutationResult = NonNullable<Awaited<ReturnType<typeof editAccount>>>
    export type EditAccountMutationBody = AccountEditRequest
    export type EditAccountMutationError = ErrorType<ApiError>
    export type EditAccountMutationVariables = {data: AccountEditRequest;headers: EditAccountHeaders}

    export const useEditAccount = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editAccount>>, TError,EditAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof editAccount>>,
        TError,
        EditAccountMutationVariables,
        TContext
      > => {
      return useMutation(getEditAccountMutationOptions(options), queryClient);
    }

export const getManageAccountKeyUrl = () => {




  return `/panel/api/v1/accounts/keys`
}

export const manageAccountKey = async (keyAccountRequest: KeyAccountRequest,
    headers: ManageAccountKeyHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<KeyAccountResult> => {

  return apiFetch<KeyAccountResult>(getManageAccountKeyUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(keyAccountRequest)
  }
);}





export const getManageAccountKeyMutationKey = () => ['manageAccountKey'] as const;

export const getManageAccountKeyMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof manageAccountKey>>, TError,ManageAccountKeyMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof manageAccountKey>>, TError,ManageAccountKeyMutationVariables, TContext> => {

const mutationKey = getManageAccountKeyMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof manageAccountKey>>, ManageAccountKeyMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  manageAccountKey(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type ManageAccountKeyMutationResult = NonNullable<Awaited<ReturnType<typeof manageAccountKey>>>
    export type ManageAccountKeyMutationBody = KeyAccountRequest
    export type ManageAccountKeyMutationError = ErrorType<ApiError>
    export type ManageAccountKeyMutationVariables = {data: KeyAccountRequest;headers: ManageAccountKeyHeaders}

    export const useManageAccountKey = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof manageAccountKey>>, TError,ManageAccountKeyMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof manageAccountKey>>,
        TError,
        ManageAccountKeyMutationVariables,
        TContext
      > => {
      return useMutation(getManageAccountKeyMutationOptions(options), queryClient);
    }

export const getGetAccountUrl = (userId: number,) => {




  return `/panel/api/v1/accounts/${userId}`
}

export const getAccount = async (userId: number, options?: Parameters<typeof apiFetch>[1]): Promise<AccountDetail> => {

  return apiFetch<AccountDetail>(getGetAccountUrl(userId),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetAccountQueryKey = (userId: number,) => {
    return [
    `/panel/api/v1/accounts/${userId}`
    ] as const;
    }


export const getGetAccountQueryOptions = <TData = Awaited<ReturnType<typeof getAccount>>, TError = ErrorType<ApiError>>(userId: number, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetAccountQueryKey(userId);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getAccount>>> = ({ signal }) => getAccount(userId, { signal, ...requestOptions });





   return  { queryKey, queryFn, enabled: userId !== null && userId !== undefined, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetAccountQueryResult = NonNullable<Awaited<ReturnType<typeof getAccount>>>
export type GetAccountQueryError = ErrorType<ApiError>


export function useGetAccount<TData = Awaited<ReturnType<typeof getAccount>>, TError = ErrorType<ApiError>>(
 userId: number, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getAccount>>,
          TError,
          Awaited<ReturnType<typeof getAccount>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAccount<TData = Awaited<ReturnType<typeof getAccount>>, TError = ErrorType<ApiError>>(
 userId: number, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getAccount>>,
          TError,
          Awaited<ReturnType<typeof getAccount>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAccount<TData = Awaited<ReturnType<typeof getAccount>>, TError = ErrorType<ApiError>>(
 userId: number, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAccount<TData = Awaited<ReturnType<typeof getAccount>>, TError = ErrorType<ApiError>>(
 userId: number, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAccount>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetAccountQueryOptions(userId,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getGetAuditHistoryUrl = (params?: GetAuditHistoryParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/audit?${stringifiedParams}` : `/panel/api/v1/audit`
}

export const getAuditHistory = async (params?: GetAuditHistoryParams, options?: Parameters<typeof apiFetch>[1]): Promise<AuditPage> => {

  return apiFetch<AuditPage>(getGetAuditHistoryUrl(params),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetAuditHistoryQueryKey = (params?: GetAuditHistoryParams,) => {
    return [
    `/panel/api/v1/audit`, ...(params ? [params] : [])
    ] as const;
    }


export const getGetAuditHistoryQueryOptions = <TData = Awaited<ReturnType<typeof getAuditHistory>>, TError = ErrorType<ApiError>>(params?: GetAuditHistoryParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetAuditHistoryQueryKey(params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getAuditHistory>>> = ({ signal }) => getAuditHistory(params, { signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetAuditHistoryQueryResult = NonNullable<Awaited<ReturnType<typeof getAuditHistory>>>
export type GetAuditHistoryQueryError = ErrorType<ApiError>


export function useGetAuditHistory<TData = Awaited<ReturnType<typeof getAuditHistory>>, TError = ErrorType<ApiError>>(
 params: undefined |  GetAuditHistoryParams, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getAuditHistory>>,
          TError,
          Awaited<ReturnType<typeof getAuditHistory>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAuditHistory<TData = Awaited<ReturnType<typeof getAuditHistory>>, TError = ErrorType<ApiError>>(
 params?: GetAuditHistoryParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getAuditHistory>>,
          TError,
          Awaited<ReturnType<typeof getAuditHistory>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAuditHistory<TData = Awaited<ReturnType<typeof getAuditHistory>>, TError = ErrorType<ApiError>>(
 params?: GetAuditHistoryParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetAuditHistory<TData = Awaited<ReturnType<typeof getAuditHistory>>, TError = ErrorType<ApiError>>(
 params?: GetAuditHistoryParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getAuditHistory>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetAuditHistoryQueryOptions(params,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListBackupsUrl = () => {




  return `/panel/api/v1/backups`
}

export const listBackups = async ( options?: Parameters<typeof apiFetch>[1]): Promise<BackupList> => {

  return apiFetch<BackupList>(getListBackupsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListBackupsQueryKey = () => {
    return [
    `/panel/api/v1/backups`
    ] as const;
    }


export const getListBackupsQueryOptions = <TData = Awaited<ReturnType<typeof listBackups>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListBackupsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listBackups>>> = ({ signal }) => listBackups({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListBackupsQueryResult = NonNullable<Awaited<ReturnType<typeof listBackups>>>
export type ListBackupsQueryError = ErrorType<ApiError>


export function useListBackups<TData = Awaited<ReturnType<typeof listBackups>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBackups>>,
          TError,
          Awaited<ReturnType<typeof listBackups>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBackups<TData = Awaited<ReturnType<typeof listBackups>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBackups>>,
          TError,
          Awaited<ReturnType<typeof listBackups>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBackups<TData = Awaited<ReturnType<typeof listBackups>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBackups<TData = Awaited<ReturnType<typeof listBackups>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBackups>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListBackupsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getCreateBackupUrl = () => {




  return `/panel/api/v1/backups`
}

export const createBackup = async (createBackupRequest: CreateBackupRequest,
    headers: CreateBackupHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<CreateBackupResponse> => {

  return apiFetch<CreateBackupResponse>(getCreateBackupUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(createBackupRequest)
  }
);}





export const getCreateBackupMutationKey = () => ['createBackup'] as const;

export const getCreateBackupMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createBackup>>, TError,CreateBackupMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof createBackup>>, TError,CreateBackupMutationVariables, TContext> => {

const mutationKey = getCreateBackupMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof createBackup>>, CreateBackupMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  createBackup(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CreateBackupMutationResult = NonNullable<Awaited<ReturnType<typeof createBackup>>>
    export type CreateBackupMutationBody = CreateBackupRequest
    export type CreateBackupMutationError = ErrorType<ApiError>
    export type CreateBackupMutationVariables = {data: CreateBackupRequest;headers: CreateBackupHeaders}

    export const useCreateBackup = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createBackup>>, TError,CreateBackupMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof createBackup>>,
        TError,
        CreateBackupMutationVariables,
        TContext
      > => {
      return useMutation(getCreateBackupMutationOptions(options), queryClient);
    }

export const getDownloadBackupUrl = () => {




  return `/panel/api/v1/backups/download`
}

export const downloadBackup = async (downloadBackupRequest: DownloadBackupRequest,
    headers: DownloadBackupHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<Blob> => {

  return apiFetch<Blob>(getDownloadBackupUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(downloadBackupRequest)
  }
);}





export const getDownloadBackupMutationKey = () => ['downloadBackup'] as const;

export const getDownloadBackupMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof downloadBackup>>, TError,DownloadBackupMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof downloadBackup>>, TError,DownloadBackupMutationVariables, TContext> => {

const mutationKey = getDownloadBackupMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof downloadBackup>>, DownloadBackupMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  downloadBackup(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type DownloadBackupMutationResult = NonNullable<Awaited<ReturnType<typeof downloadBackup>>>
    export type DownloadBackupMutationBody = DownloadBackupRequest
    export type DownloadBackupMutationError = ErrorType<ApiError>
    export type DownloadBackupMutationVariables = {data: DownloadBackupRequest;headers: DownloadBackupHeaders}

    export const useDownloadBackup = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof downloadBackup>>, TError,DownloadBackupMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof downloadBackup>>,
        TError,
        DownloadBackupMutationVariables,
        TContext
      > => {
      return useMutation(getDownloadBackupMutationOptions(options), queryClient);
    }

export const getListBanlistsUrl = () => {




  return `/panel/api/v1/banlists`
}

export const listBanlists = async ( options?: Parameters<typeof apiFetch>[1]): Promise<BanlistChoices> => {

  return apiFetch<BanlistChoices>(getListBanlistsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListBanlistsQueryKey = () => {
    return [
    `/panel/api/v1/banlists`
    ] as const;
    }


export const getListBanlistsQueryOptions = <TData = Awaited<ReturnType<typeof listBanlists>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListBanlistsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listBanlists>>> = ({ signal }) => listBanlists({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListBanlistsQueryResult = NonNullable<Awaited<ReturnType<typeof listBanlists>>>
export type ListBanlistsQueryError = ErrorType<ApiError>


export function useListBanlists<TData = Awaited<ReturnType<typeof listBanlists>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBanlists>>,
          TError,
          Awaited<ReturnType<typeof listBanlists>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBanlists<TData = Awaited<ReturnType<typeof listBanlists>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBanlists>>,
          TError,
          Awaited<ReturnType<typeof listBanlists>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBanlists<TData = Awaited<ReturnType<typeof listBanlists>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBanlists<TData = Awaited<ReturnType<typeof listBanlists>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBanlists>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListBanlistsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListBpCodesUrl = () => {




  return `/panel/api/v1/bp-codes`
}

export const listBpCodes = async ( options?: Parameters<typeof apiFetch>[1]): Promise<BpCodeList> => {

  return apiFetch<BpCodeList>(getListBpCodesUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListBpCodesQueryKey = () => {
    return [
    `/panel/api/v1/bp-codes`
    ] as const;
    }


export const getListBpCodesQueryOptions = <TData = Awaited<ReturnType<typeof listBpCodes>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListBpCodesQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listBpCodes>>> = ({ signal }) => listBpCodes({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListBpCodesQueryResult = NonNullable<Awaited<ReturnType<typeof listBpCodes>>>
export type ListBpCodesQueryError = ErrorType<ApiError>


export function useListBpCodes<TData = Awaited<ReturnType<typeof listBpCodes>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBpCodes>>,
          TError,
          Awaited<ReturnType<typeof listBpCodes>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBpCodes<TData = Awaited<ReturnType<typeof listBpCodes>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listBpCodes>>,
          TError,
          Awaited<ReturnType<typeof listBpCodes>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBpCodes<TData = Awaited<ReturnType<typeof listBpCodes>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListBpCodes<TData = Awaited<ReturnType<typeof listBpCodes>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listBpCodes>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListBpCodesQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getCreateBpCodeUrl = () => {




  return `/panel/api/v1/bp-codes`
}

export const createBpCode = async (createBpCodeRequest: CreateBpCodeRequest,
    headers: CreateBpCodeHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<CreateBpCodeResult> => {

  return apiFetch<CreateBpCodeResult>(getCreateBpCodeUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(createBpCodeRequest)
  }
);}





export const getCreateBpCodeMutationKey = () => ['createBpCode'] as const;

export const getCreateBpCodeMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createBpCode>>, TError,CreateBpCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof createBpCode>>, TError,CreateBpCodeMutationVariables, TContext> => {

const mutationKey = getCreateBpCodeMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof createBpCode>>, CreateBpCodeMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  createBpCode(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CreateBpCodeMutationResult = NonNullable<Awaited<ReturnType<typeof createBpCode>>>
    export type CreateBpCodeMutationBody = CreateBpCodeRequest
    export type CreateBpCodeMutationError = ErrorType<ApiError>
    export type CreateBpCodeMutationVariables = {data: CreateBpCodeRequest;headers: CreateBpCodeHeaders}

    export const useCreateBpCode = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createBpCode>>, TError,CreateBpCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof createBpCode>>,
        TError,
        CreateBpCodeMutationVariables,
        TContext
      > => {
      return useMutation(getCreateBpCodeMutationOptions(options), queryClient);
    }

export const getRemoveBpCodeUrl = () => {




  return `/panel/api/v1/bp-codes/remove`
}

export const removeBpCode = async (removeBpCodeRequest: RemoveBpCodeRequest,
    headers: RemoveBpCodeHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<RemoveBpCodeResult> => {

  return apiFetch<RemoveBpCodeResult>(getRemoveBpCodeUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(removeBpCodeRequest)
  }
);}





export const getRemoveBpCodeMutationKey = () => ['removeBpCode'] as const;

export const getRemoveBpCodeMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof removeBpCode>>, TError,RemoveBpCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof removeBpCode>>, TError,RemoveBpCodeMutationVariables, TContext> => {

const mutationKey = getRemoveBpCodeMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof removeBpCode>>, RemoveBpCodeMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  removeBpCode(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type RemoveBpCodeMutationResult = NonNullable<Awaited<ReturnType<typeof removeBpCode>>>
    export type RemoveBpCodeMutationBody = RemoveBpCodeRequest
    export type RemoveBpCodeMutationError = ErrorType<ApiError>
    export type RemoveBpCodeMutationVariables = {data: RemoveBpCodeRequest;headers: RemoveBpCodeHeaders}

    export const useRemoveBpCode = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof removeBpCode>>, TError,RemoveBpCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof removeBpCode>>,
        TError,
        RemoveBpCodeMutationVariables,
        TContext
      > => {
      return useMutation(getRemoveBpCodeMutationOptions(options), queryClient);
    }

export const getGetCardImageUrl = (row: number,
    size: CardImageSize,
    params: GetCardImageParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/cards/${row}/${size}?${stringifiedParams}` : `/panel/api/v1/cards/${row}/${size}`
}

export const getCardImage = async (row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers?: GetCardImageHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<Blob> => {

  return apiFetch<Blob>(getGetCardImageUrl(row,size,params),
  {
    ...options,
    method: 'GET',
    headers: getHeaders(headers, options?.headers)

  }
);}





export const getGetCardImageQueryKey = (row: number,
    size: CardImageSize,
    params?: GetCardImageParams,) => {
    return [
    `/panel/api/v1/cards/${row}/${size}`, ...(params ? [params] : [])
    ] as const;
    }


export const getGetCardImageQueryOptions = <TData = Awaited<ReturnType<typeof getCardImage>>, TError = ErrorType<void | ApiError>>(row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers?: GetCardImageHeaders, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetCardImageQueryKey(row,size,params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getCardImage>>> = ({ signal }) => getCardImage(row,size,params,headers, { signal, ...requestOptions });





   return  { queryKey, queryFn, enabled: row !== null && row !== undefined && size !== null && size !== undefined, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetCardImageQueryResult = NonNullable<Awaited<ReturnType<typeof getCardImage>>>
export type GetCardImageQueryError = ErrorType<void | ApiError>


export function useGetCardImage<TData = Awaited<ReturnType<typeof getCardImage>>, TError = ErrorType<void | ApiError>>(
 row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers: undefined |  GetCardImageHeaders, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getCardImage>>,
          TError,
          Awaited<ReturnType<typeof getCardImage>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetCardImage<TData = Awaited<ReturnType<typeof getCardImage>>, TError = ErrorType<void | ApiError>>(
 row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers?: GetCardImageHeaders, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getCardImage>>,
          TError,
          Awaited<ReturnType<typeof getCardImage>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetCardImage<TData = Awaited<ReturnType<typeof getCardImage>>, TError = ErrorType<void | ApiError>>(
 row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers?: GetCardImageHeaders, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetCardImage<TData = Awaited<ReturnType<typeof getCardImage>>, TError = ErrorType<void | ApiError>>(
 row: number,
    size: CardImageSize,
    params: GetCardImageParams,
    headers?: GetCardImageHeaders, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getCardImage>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetCardImageQueryOptions(row,size,params,headers,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListChampionshipsUrl = () => {




  return `/panel/api/v1/championships`
}

export const listChampionships = async ( options?: Parameters<typeof apiFetch>[1]): Promise<ChampionshipList> => {

  return apiFetch<ChampionshipList>(getListChampionshipsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListChampionshipsQueryKey = () => {
    return [
    `/panel/api/v1/championships`
    ] as const;
    }


export const getListChampionshipsQueryOptions = <TData = Awaited<ReturnType<typeof listChampionships>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListChampionshipsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listChampionships>>> = ({ signal }) => listChampionships({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListChampionshipsQueryResult = NonNullable<Awaited<ReturnType<typeof listChampionships>>>
export type ListChampionshipsQueryError = ErrorType<ApiError>


export function useListChampionships<TData = Awaited<ReturnType<typeof listChampionships>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listChampionships>>,
          TError,
          Awaited<ReturnType<typeof listChampionships>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListChampionships<TData = Awaited<ReturnType<typeof listChampionships>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listChampionships>>,
          TError,
          Awaited<ReturnType<typeof listChampionships>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListChampionships<TData = Awaited<ReturnType<typeof listChampionships>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListChampionships<TData = Awaited<ReturnType<typeof listChampionships>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listChampionships>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListChampionshipsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getCancelChampionshipUrl = () => {




  return `/panel/api/v1/championships/cancel`
}

export const cancelChampionship = async (cancelChampionshipRequest: CancelChampionshipRequest,
    headers: CancelChampionshipHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<ChampionshipResult> => {

  return apiFetch<ChampionshipResult>(getCancelChampionshipUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(cancelChampionshipRequest)
  }
);}





export const getCancelChampionshipMutationKey = () => ['cancelChampionship'] as const;

export const getCancelChampionshipMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof cancelChampionship>>, TError,CancelChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof cancelChampionship>>, TError,CancelChampionshipMutationVariables, TContext> => {

const mutationKey = getCancelChampionshipMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof cancelChampionship>>, CancelChampionshipMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  cancelChampionship(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CancelChampionshipMutationResult = NonNullable<Awaited<ReturnType<typeof cancelChampionship>>>
    export type CancelChampionshipMutationBody = CancelChampionshipRequest
    export type CancelChampionshipMutationError = ErrorType<ApiError>
    export type CancelChampionshipMutationVariables = {data: CancelChampionshipRequest;headers: CancelChampionshipHeaders}

    export const useCancelChampionship = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof cancelChampionship>>, TError,CancelChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof cancelChampionship>>,
        TError,
        CancelChampionshipMutationVariables,
        TContext
      > => {
      return useMutation(getCancelChampionshipMutationOptions(options), queryClient);
    }

export const getCreateChampionshipUrl = () => {




  return `/panel/api/v1/championships/create`
}

export const createChampionship = async (createChampionshipRequest: CreateChampionshipRequest,
    headers: CreateChampionshipHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<ChampionshipResult> => {

  return apiFetch<ChampionshipResult>(getCreateChampionshipUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(createChampionshipRequest)
  }
);}





export const getCreateChampionshipMutationKey = () => ['createChampionship'] as const;

export const getCreateChampionshipMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createChampionship>>, TError,CreateChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof createChampionship>>, TError,CreateChampionshipMutationVariables, TContext> => {

const mutationKey = getCreateChampionshipMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof createChampionship>>, CreateChampionshipMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  createChampionship(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CreateChampionshipMutationResult = NonNullable<Awaited<ReturnType<typeof createChampionship>>>
    export type CreateChampionshipMutationBody = CreateChampionshipRequest
    export type CreateChampionshipMutationError = ErrorType<ApiError>
    export type CreateChampionshipMutationVariables = {data: CreateChampionshipRequest;headers: CreateChampionshipHeaders}

    export const useCreateChampionship = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createChampionship>>, TError,CreateChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof createChampionship>>,
        TError,
        CreateChampionshipMutationVariables,
        TContext
      > => {
      return useMutation(getCreateChampionshipMutationOptions(options), queryClient);
    }

export const getEditChampionshipUrl = () => {




  return `/panel/api/v1/championships/edit`
}

export const editChampionship = async (editChampionshipRequest: EditChampionshipRequest,
    headers: EditChampionshipHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<ChampionshipResult> => {

  return apiFetch<ChampionshipResult>(getEditChampionshipUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(editChampionshipRequest)
  }
);}





export const getEditChampionshipMutationKey = () => ['editChampionship'] as const;

export const getEditChampionshipMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editChampionship>>, TError,EditChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof editChampionship>>, TError,EditChampionshipMutationVariables, TContext> => {

const mutationKey = getEditChampionshipMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof editChampionship>>, EditChampionshipMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  editChampionship(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type EditChampionshipMutationResult = NonNullable<Awaited<ReturnType<typeof editChampionship>>>
    export type EditChampionshipMutationBody = EditChampionshipRequest
    export type EditChampionshipMutationError = ErrorType<ApiError>
    export type EditChampionshipMutationVariables = {data: EditChampionshipRequest;headers: EditChampionshipHeaders}

    export const useEditChampionship = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editChampionship>>, TError,EditChampionshipMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof editChampionship>>,
        TError,
        EditChampionshipMutationVariables,
        TContext
      > => {
      return useMutation(getEditChampionshipMutationOptions(options), queryClient);
    }

export const getListCountriesUrl = () => {




  return `/panel/api/v1/countries`
}

export const listCountries = async ( options?: Parameters<typeof apiFetch>[1]): Promise<CountryList> => {

  return apiFetch<CountryList>(getListCountriesUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListCountriesQueryKey = () => {
    return [
    `/panel/api/v1/countries`
    ] as const;
    }


export const getListCountriesQueryOptions = <TData = Awaited<ReturnType<typeof listCountries>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListCountriesQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listCountries>>> = ({ signal }) => listCountries({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListCountriesQueryResult = NonNullable<Awaited<ReturnType<typeof listCountries>>>
export type ListCountriesQueryError = ErrorType<ApiError>


export function useListCountries<TData = Awaited<ReturnType<typeof listCountries>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listCountries>>,
          TError,
          Awaited<ReturnType<typeof listCountries>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListCountries<TData = Awaited<ReturnType<typeof listCountries>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listCountries>>,
          TError,
          Awaited<ReturnType<typeof listCountries>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListCountries<TData = Awaited<ReturnType<typeof listCountries>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListCountries<TData = Awaited<ReturnType<typeof listCountries>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listCountries>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListCountriesQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListEventsUrl = () => {




  return `/panel/api/v1/events`
}

export const listEvents = async ( options?: Parameters<typeof apiFetch>[1]): Promise<EventList> => {

  return apiFetch<EventList>(getListEventsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListEventsQueryKey = () => {
    return [
    `/panel/api/v1/events`
    ] as const;
    }


export const getListEventsQueryOptions = <TData = Awaited<ReturnType<typeof listEvents>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListEventsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listEvents>>> = ({ signal }) => listEvents({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListEventsQueryResult = NonNullable<Awaited<ReturnType<typeof listEvents>>>
export type ListEventsQueryError = ErrorType<ApiError>


export function useListEvents<TData = Awaited<ReturnType<typeof listEvents>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listEvents>>,
          TError,
          Awaited<ReturnType<typeof listEvents>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListEvents<TData = Awaited<ReturnType<typeof listEvents>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listEvents>>,
          TError,
          Awaited<ReturnType<typeof listEvents>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListEvents<TData = Awaited<ReturnType<typeof listEvents>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListEvents<TData = Awaited<ReturnType<typeof listEvents>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listEvents>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListEventsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getCancelEventUrl = () => {




  return `/panel/api/v1/events/cancel`
}

export const cancelEvent = async (cancelEventRequest: CancelEventRequest,
    headers: CancelEventHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<EventResult> => {

  return apiFetch<EventResult>(getCancelEventUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(cancelEventRequest)
  }
);}





export const getCancelEventMutationKey = () => ['cancelEvent'] as const;

export const getCancelEventMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof cancelEvent>>, TError,CancelEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof cancelEvent>>, TError,CancelEventMutationVariables, TContext> => {

const mutationKey = getCancelEventMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof cancelEvent>>, CancelEventMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  cancelEvent(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CancelEventMutationResult = NonNullable<Awaited<ReturnType<typeof cancelEvent>>>
    export type CancelEventMutationBody = CancelEventRequest
    export type CancelEventMutationError = ErrorType<ApiError>
    export type CancelEventMutationVariables = {data: CancelEventRequest;headers: CancelEventHeaders}

    export const useCancelEvent = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof cancelEvent>>, TError,CancelEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof cancelEvent>>,
        TError,
        CancelEventMutationVariables,
        TContext
      > => {
      return useMutation(getCancelEventMutationOptions(options), queryClient);
    }

export const getCreateEventUrl = () => {




  return `/panel/api/v1/events/create`
}

export const createEvent = async (createEventRequest: CreateEventRequest,
    headers: CreateEventHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<EventResult> => {

  return apiFetch<EventResult>(getCreateEventUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(createEventRequest)
  }
);}





export const getCreateEventMutationKey = () => ['createEvent'] as const;

export const getCreateEventMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createEvent>>, TError,CreateEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof createEvent>>, TError,CreateEventMutationVariables, TContext> => {

const mutationKey = getCreateEventMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof createEvent>>, CreateEventMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  createEvent(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CreateEventMutationResult = NonNullable<Awaited<ReturnType<typeof createEvent>>>
    export type CreateEventMutationBody = CreateEventRequest
    export type CreateEventMutationError = ErrorType<ApiError>
    export type CreateEventMutationVariables = {data: CreateEventRequest;headers: CreateEventHeaders}

    export const useCreateEvent = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createEvent>>, TError,CreateEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof createEvent>>,
        TError,
        CreateEventMutationVariables,
        TContext
      > => {
      return useMutation(getCreateEventMutationOptions(options), queryClient);
    }

export const getEditEventUrl = () => {




  return `/panel/api/v1/events/edit`
}

export const editEvent = async (editEventRequest: EditEventRequest,
    headers: EditEventHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<EventResult> => {

  return apiFetch<EventResult>(getEditEventUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(editEventRequest)
  }
);}





export const getEditEventMutationKey = () => ['editEvent'] as const;

export const getEditEventMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editEvent>>, TError,EditEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof editEvent>>, TError,EditEventMutationVariables, TContext> => {

const mutationKey = getEditEventMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof editEvent>>, EditEventMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  editEvent(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type EditEventMutationResult = NonNullable<Awaited<ReturnType<typeof editEvent>>>
    export type EditEventMutationBody = EditEventRequest
    export type EditEventMutationError = ErrorType<ApiError>
    export type EditEventMutationVariables = {data: EditEventRequest;headers: EditEventHeaders}

    export const useEditEvent = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof editEvent>>, TError,EditEventMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof editEvent>>,
        TError,
        EditEventMutationVariables,
        TContext
      > => {
      return useMutation(getEditEventMutationOptions(options), queryClient);
    }

export const getGivePlayerGiftUrl = () => {




  return `/panel/api/v1/gifts`
}

export const givePlayerGift = async (giftRequest: GiftRequest,
    headers: GivePlayerGiftHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<GiftResult> => {

  return apiFetch<GiftResult>(getGivePlayerGiftUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(giftRequest)
  }
);}





export const getGivePlayerGiftMutationKey = () => ['givePlayerGift'] as const;

export const getGivePlayerGiftMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof givePlayerGift>>, TError,GivePlayerGiftMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof givePlayerGift>>, TError,GivePlayerGiftMutationVariables, TContext> => {

const mutationKey = getGivePlayerGiftMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof givePlayerGift>>, GivePlayerGiftMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  givePlayerGift(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type GivePlayerGiftMutationResult = NonNullable<Awaited<ReturnType<typeof givePlayerGift>>>
    export type GivePlayerGiftMutationBody = GiftRequest
    export type GivePlayerGiftMutationError = ErrorType<ApiError>
    export type GivePlayerGiftMutationVariables = {data: GiftRequest;headers: GivePlayerGiftHeaders}

    export const useGivePlayerGift = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof givePlayerGift>>, TError,GivePlayerGiftMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof givePlayerGift>>,
        TError,
        GivePlayerGiftMutationVariables,
        TContext
      > => {
      return useMutation(getGivePlayerGiftMutationOptions(options), queryClient);
    }

export const getListGiftChoicesUrl = (params?: ListGiftChoicesParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/gifts/choices?${stringifiedParams}` : `/panel/api/v1/gifts/choices`
}

export const listGiftChoices = async (params?: ListGiftChoicesParams, options?: Parameters<typeof apiFetch>[1]): Promise<GiftChoices> => {

  return apiFetch<GiftChoices>(getListGiftChoicesUrl(params),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListGiftChoicesQueryKey = (params?: ListGiftChoicesParams,) => {
    return [
    `/panel/api/v1/gifts/choices`, ...(params ? [params] : [])
    ] as const;
    }


export const getListGiftChoicesQueryOptions = <TData = Awaited<ReturnType<typeof listGiftChoices>>, TError = ErrorType<ApiError>>(params?: ListGiftChoicesParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListGiftChoicesQueryKey(params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listGiftChoices>>> = ({ signal }) => listGiftChoices(params, { signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListGiftChoicesQueryResult = NonNullable<Awaited<ReturnType<typeof listGiftChoices>>>
export type ListGiftChoicesQueryError = ErrorType<ApiError>


export function useListGiftChoices<TData = Awaited<ReturnType<typeof listGiftChoices>>, TError = ErrorType<ApiError>>(
 params: undefined |  ListGiftChoicesParams, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listGiftChoices>>,
          TError,
          Awaited<ReturnType<typeof listGiftChoices>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListGiftChoices<TData = Awaited<ReturnType<typeof listGiftChoices>>, TError = ErrorType<ApiError>>(
 params?: ListGiftChoicesParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listGiftChoices>>,
          TError,
          Awaited<ReturnType<typeof listGiftChoices>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListGiftChoices<TData = Awaited<ReturnType<typeof listGiftChoices>>, TError = ErrorType<ApiError>>(
 params?: ListGiftChoicesParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListGiftChoices<TData = Awaited<ReturnType<typeof listGiftChoices>>, TError = ErrorType<ApiError>>(
 params?: ListGiftChoicesParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listGiftChoices>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListGiftChoicesQueryOptions(params,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getGiveBpUrl = () => {




  return `/panel/api/v1/give-bp`
}

export const giveBp = async (giveBpByIdRequest: GiveBpByIdRequest,
    headers: GiveBpHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<GiveBpResult> => {

  return apiFetch<GiveBpResult>(getGiveBpUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(giveBpByIdRequest)
  }
);}





export const getGiveBpMutationKey = () => ['giveBp'] as const;

export const getGiveBpMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof giveBp>>, TError,GiveBpMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof giveBp>>, TError,GiveBpMutationVariables, TContext> => {

const mutationKey = getGiveBpMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof giveBp>>, GiveBpMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  giveBp(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type GiveBpMutationResult = NonNullable<Awaited<ReturnType<typeof giveBp>>>
    export type GiveBpMutationBody = GiveBpByIdRequest
    export type GiveBpMutationError = ErrorType<ApiError>
    export type GiveBpMutationVariables = {data: GiveBpByIdRequest;headers: GiveBpHeaders}

    export const useGiveBp = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof giveBp>>, TError,GiveBpMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof giveBp>>,
        TError,
        GiveBpMutationVariables,
        TContext
      > => {
      return useMutation(getGiveBpMutationOptions(options), queryClient);
    }

export const getListInvitesUrl = () => {




  return `/panel/api/v1/invites`
}

export const listInvites = async ( options?: Parameters<typeof apiFetch>[1]): Promise<InviteList> => {

  return apiFetch<InviteList>(getListInvitesUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListInvitesQueryKey = () => {
    return [
    `/panel/api/v1/invites`
    ] as const;
    }


export const getListInvitesQueryOptions = <TData = Awaited<ReturnType<typeof listInvites>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListInvitesQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listInvites>>> = ({ signal }) => listInvites({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListInvitesQueryResult = NonNullable<Awaited<ReturnType<typeof listInvites>>>
export type ListInvitesQueryError = ErrorType<ApiError>


export function useListInvites<TData = Awaited<ReturnType<typeof listInvites>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listInvites>>,
          TError,
          Awaited<ReturnType<typeof listInvites>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListInvites<TData = Awaited<ReturnType<typeof listInvites>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listInvites>>,
          TError,
          Awaited<ReturnType<typeof listInvites>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListInvites<TData = Awaited<ReturnType<typeof listInvites>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListInvites<TData = Awaited<ReturnType<typeof listInvites>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listInvites>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListInvitesQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getCreateInviteUrl = () => {




  return `/panel/api/v1/invites`
}

export const createInvite = async (createInviteRequest: CreateInviteRequest,
    headers: CreateInviteHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<CreateInviteResult> => {

  return apiFetch<CreateInviteResult>(getCreateInviteUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(createInviteRequest)
  }
);}





export const getCreateInviteMutationKey = () => ['createInvite'] as const;

export const getCreateInviteMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createInvite>>, TError,CreateInviteMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof createInvite>>, TError,CreateInviteMutationVariables, TContext> => {

const mutationKey = getCreateInviteMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof createInvite>>, CreateInviteMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  createInvite(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type CreateInviteMutationResult = NonNullable<Awaited<ReturnType<typeof createInvite>>>
    export type CreateInviteMutationBody = CreateInviteRequest
    export type CreateInviteMutationError = ErrorType<ApiError>
    export type CreateInviteMutationVariables = {data: CreateInviteRequest;headers: CreateInviteHeaders}

    export const useCreateInvite = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof createInvite>>, TError,CreateInviteMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof createInvite>>,
        TError,
        CreateInviteMutationVariables,
        TContext
      > => {
      return useMutation(getCreateInviteMutationOptions(options), queryClient);
    }

export const getGetServerLogsUrl = (params?: GetServerLogsParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/logs?${stringifiedParams}` : `/panel/api/v1/logs`
}

export const getServerLogs = async (params?: GetServerLogsParams, options?: Parameters<typeof apiFetch>[1]): Promise<LogPage> => {

  return apiFetch<LogPage>(getGetServerLogsUrl(params),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetServerLogsQueryKey = (params?: GetServerLogsParams,) => {
    return [
    `/panel/api/v1/logs`, ...(params ? [params] : [])
    ] as const;
    }


export const getGetServerLogsQueryOptions = <TData = Awaited<ReturnType<typeof getServerLogs>>, TError = ErrorType<ApiError>>(params?: GetServerLogsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetServerLogsQueryKey(params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getServerLogs>>> = ({ signal }) => getServerLogs(params, { signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetServerLogsQueryResult = NonNullable<Awaited<ReturnType<typeof getServerLogs>>>
export type GetServerLogsQueryError = ErrorType<ApiError>


export function useGetServerLogs<TData = Awaited<ReturnType<typeof getServerLogs>>, TError = ErrorType<ApiError>>(
 params: undefined |  GetServerLogsParams, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getServerLogs>>,
          TError,
          Awaited<ReturnType<typeof getServerLogs>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerLogs<TData = Awaited<ReturnType<typeof getServerLogs>>, TError = ErrorType<ApiError>>(
 params?: GetServerLogsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getServerLogs>>,
          TError,
          Awaited<ReturnType<typeof getServerLogs>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerLogs<TData = Awaited<ReturnType<typeof getServerLogs>>, TError = ErrorType<ApiError>>(
 params?: GetServerLogsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerLogs<TData = Awaited<ReturnType<typeof getServerLogs>>, TError = ErrorType<ApiError>>(
 params?: GetServerLogsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerLogs>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetServerLogsQueryOptions(params,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getStartMfaEnrollmentUrl = () => {




  return `/panel/api/v1/mfa/enrollment`
}

export const startMfaEnrollment = async (headers: StartMfaEnrollmentHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<MfaEnrollmentStart> => {

  return apiFetch<MfaEnrollmentStart>(getStartMfaEnrollmentUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders(headers, options?.headers)

  }
);}





export const getStartMfaEnrollmentMutationKey = () => ['startMfaEnrollment'] as const;

export const getStartMfaEnrollmentMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof startMfaEnrollment>>, TError,StartMfaEnrollmentMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof startMfaEnrollment>>, TError,StartMfaEnrollmentMutationVariables, TContext> => {

const mutationKey = getStartMfaEnrollmentMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof startMfaEnrollment>>, StartMfaEnrollmentMutationVariables> = (props) => {
          const {headers} = props ?? {};

          return  startMfaEnrollment(headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type StartMfaEnrollmentMutationResult = NonNullable<Awaited<ReturnType<typeof startMfaEnrollment>>>

    export type StartMfaEnrollmentMutationError = ErrorType<ApiError>
    export type StartMfaEnrollmentMutationVariables = {headers: StartMfaEnrollmentHeaders}

    export const useStartMfaEnrollment = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof startMfaEnrollment>>, TError,StartMfaEnrollmentMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof startMfaEnrollment>>,
        TError,
        StartMfaEnrollmentMutationVariables,
        TContext
      > => {
      return useMutation(getStartMfaEnrollmentMutationOptions(options), queryClient);
    }

export const getConfirmMfaEnrollmentUrl = () => {




  return `/panel/api/v1/mfa/enrollment/confirm`
}

export const confirmMfaEnrollment = async (codeRequest: CodeRequest,
    headers: ConfirmMfaEnrollmentHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<MfaEnrolled> => {

  return apiFetch<MfaEnrolled>(getConfirmMfaEnrollmentUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(codeRequest)
  }
);}





export const getConfirmMfaEnrollmentMutationKey = () => ['confirmMfaEnrollment'] as const;

export const getConfirmMfaEnrollmentMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof confirmMfaEnrollment>>, TError,ConfirmMfaEnrollmentMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof confirmMfaEnrollment>>, TError,ConfirmMfaEnrollmentMutationVariables, TContext> => {

const mutationKey = getConfirmMfaEnrollmentMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof confirmMfaEnrollment>>, ConfirmMfaEnrollmentMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  confirmMfaEnrollment(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type ConfirmMfaEnrollmentMutationResult = NonNullable<Awaited<ReturnType<typeof confirmMfaEnrollment>>>
    export type ConfirmMfaEnrollmentMutationBody = CodeRequest
    export type ConfirmMfaEnrollmentMutationError = ErrorType<ApiError>
    export type ConfirmMfaEnrollmentMutationVariables = {data: CodeRequest;headers: ConfirmMfaEnrollmentHeaders}

    export const useConfirmMfaEnrollment = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof confirmMfaEnrollment>>, TError,ConfirmMfaEnrollmentMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof confirmMfaEnrollment>>,
        TError,
        ConfirmMfaEnrollmentMutationVariables,
        TContext
      > => {
      return useMutation(getConfirmMfaEnrollmentMutationOptions(options), queryClient);
    }

export const getRegenerateRecoveryCodesUrl = () => {




  return `/panel/api/v1/mfa/recovery-codes`
}

export const regenerateRecoveryCodes = async (emptyRequest: EmptyRequest,
    headers: RegenerateRecoveryCodesHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<RecoveryCodes> => {

  return apiFetch<RecoveryCodes>(getRegenerateRecoveryCodesUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(emptyRequest)
  }
);}





export const getRegenerateRecoveryCodesMutationKey = () => ['regenerateRecoveryCodes'] as const;

export const getRegenerateRecoveryCodesMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof regenerateRecoveryCodes>>, TError,RegenerateRecoveryCodesMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof regenerateRecoveryCodes>>, TError,RegenerateRecoveryCodesMutationVariables, TContext> => {

const mutationKey = getRegenerateRecoveryCodesMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof regenerateRecoveryCodes>>, RegenerateRecoveryCodesMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  regenerateRecoveryCodes(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type RegenerateRecoveryCodesMutationResult = NonNullable<Awaited<ReturnType<typeof regenerateRecoveryCodes>>>
    export type RegenerateRecoveryCodesMutationBody = EmptyRequest
    export type RegenerateRecoveryCodesMutationError = ErrorType<ApiError>
    export type RegenerateRecoveryCodesMutationVariables = {data: EmptyRequest;headers: RegenerateRecoveryCodesHeaders}

    export const useRegenerateRecoveryCodes = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof regenerateRecoveryCodes>>, TError,RegenerateRecoveryCodesMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof regenerateRecoveryCodes>>,
        TError,
        RegenerateRecoveryCodesMutationVariables,
        TContext
      > => {
      return useMutation(getRegenerateRecoveryCodesMutationOptions(options), queryClient);
    }

export const getConfirmMfaStepUpUrl = () => {




  return `/panel/api/v1/mfa/step-up`
}

export const confirmMfaStepUp = async (codeRequest: CodeRequest,
    headers: ConfirmMfaStepUpHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<StaffSession> => {

  return apiFetch<StaffSession>(getConfirmMfaStepUpUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(codeRequest)
  }
);}





export const getConfirmMfaStepUpMutationKey = () => ['confirmMfaStepUp'] as const;

export const getConfirmMfaStepUpMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof confirmMfaStepUp>>, TError,ConfirmMfaStepUpMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof confirmMfaStepUp>>, TError,ConfirmMfaStepUpMutationVariables, TContext> => {

const mutationKey = getConfirmMfaStepUpMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof confirmMfaStepUp>>, ConfirmMfaStepUpMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  confirmMfaStepUp(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type ConfirmMfaStepUpMutationResult = NonNullable<Awaited<ReturnType<typeof confirmMfaStepUp>>>
    export type ConfirmMfaStepUpMutationBody = CodeRequest
    export type ConfirmMfaStepUpMutationError = ErrorType<ApiError>
    export type ConfirmMfaStepUpMutationVariables = {data: CodeRequest;headers: ConfirmMfaStepUpHeaders}

    export const useConfirmMfaStepUp = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof confirmMfaStepUp>>, TError,ConfirmMfaStepUpMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof confirmMfaStepUp>>,
        TError,
        ConfirmMfaStepUpMutationVariables,
        TContext
      > => {
      return useMutation(getConfirmMfaStepUpMutationOptions(options), queryClient);
    }

export const getModerateAccountUrl = () => {




  return `/panel/api/v1/moderation`
}

export const moderateAccount = async (moderationRequest: ModerationRequest,
    headers: ModerateAccountHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<ModerationResponse> => {

  return apiFetch<ModerationResponse>(getModerateAccountUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(moderationRequest)
  }
);}





export const getModerateAccountMutationKey = () => ['moderateAccount'] as const;

export const getModerateAccountMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof moderateAccount>>, TError,ModerateAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof moderateAccount>>, TError,ModerateAccountMutationVariables, TContext> => {

const mutationKey = getModerateAccountMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof moderateAccount>>, ModerateAccountMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  moderateAccount(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type ModerateAccountMutationResult = NonNullable<Awaited<ReturnType<typeof moderateAccount>>>
    export type ModerateAccountMutationBody = ModerationRequest
    export type ModerateAccountMutationError = ErrorType<ApiError>
    export type ModerateAccountMutationVariables = {data: ModerationRequest;headers: ModerateAccountHeaders}

    export const useModerateAccount = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof moderateAccount>>, TError,ModerateAccountMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof moderateAccount>>,
        TError,
        ModerateAccountMutationVariables,
        TContext
      > => {
      return useMutation(getModerateAccountMutationOptions(options), queryClient);
    }

export const getListProblemReportsUrl = (params?: ListProblemReportsParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/problem-reports?${stringifiedParams}` : `/panel/api/v1/problem-reports`
}

export const listProblemReports = async (params?: ListProblemReportsParams, options?: Parameters<typeof apiFetch>[1]): Promise<ProblemReportList> => {

  return apiFetch<ProblemReportList>(getListProblemReportsUrl(params),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListProblemReportsQueryKey = (params?: ListProblemReportsParams,) => {
    return [
    `/panel/api/v1/problem-reports`, ...(params ? [params] : [])
    ] as const;
    }


export const getListProblemReportsQueryOptions = <TData = Awaited<ReturnType<typeof listProblemReports>>, TError = ErrorType<ApiError>>(params?: ListProblemReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListProblemReportsQueryKey(params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listProblemReports>>> = ({ signal }) => listProblemReports(params, { signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListProblemReportsQueryResult = NonNullable<Awaited<ReturnType<typeof listProblemReports>>>
export type ListProblemReportsQueryError = ErrorType<ApiError>


export function useListProblemReports<TData = Awaited<ReturnType<typeof listProblemReports>>, TError = ErrorType<ApiError>>(
 params: undefined |  ListProblemReportsParams, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listProblemReports>>,
          TError,
          Awaited<ReturnType<typeof listProblemReports>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListProblemReports<TData = Awaited<ReturnType<typeof listProblemReports>>, TError = ErrorType<ApiError>>(
 params?: ListProblemReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listProblemReports>>,
          TError,
          Awaited<ReturnType<typeof listProblemReports>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListProblemReports<TData = Awaited<ReturnType<typeof listProblemReports>>, TError = ErrorType<ApiError>>(
 params?: ListProblemReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListProblemReports<TData = Awaited<ReturnType<typeof listProblemReports>>, TError = ErrorType<ApiError>>(
 params?: ListProblemReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listProblemReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListProblemReportsQueryOptions(params,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getGetProblemReportUrl = (id: string,) => {




  return `/panel/api/v1/problem-reports/${id}`
}

export const getProblemReport = async (id: string, options?: Parameters<typeof apiFetch>[1]): Promise<ProblemReport> => {

  return apiFetch<ProblemReport>(getGetProblemReportUrl(id),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetProblemReportQueryKey = (id: string,) => {
    return [
    `/panel/api/v1/problem-reports/${id}`
    ] as const;
    }


export const getGetProblemReportQueryOptions = <TData = Awaited<ReturnType<typeof getProblemReport>>, TError = ErrorType<ApiError>>(id: string, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetProblemReportQueryKey(id);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getProblemReport>>> = ({ signal }) => getProblemReport(id, { signal, ...requestOptions });





   return  { queryKey, queryFn, enabled: id !== null && id !== undefined, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetProblemReportQueryResult = NonNullable<Awaited<ReturnType<typeof getProblemReport>>>
export type GetProblemReportQueryError = ErrorType<ApiError>


export function useGetProblemReport<TData = Awaited<ReturnType<typeof getProblemReport>>, TError = ErrorType<ApiError>>(
 id: string, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getProblemReport>>,
          TError,
          Awaited<ReturnType<typeof getProblemReport>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetProblemReport<TData = Awaited<ReturnType<typeof getProblemReport>>, TError = ErrorType<ApiError>>(
 id: string, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getProblemReport>>,
          TError,
          Awaited<ReturnType<typeof getProblemReport>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetProblemReport<TData = Awaited<ReturnType<typeof getProblemReport>>, TError = ErrorType<ApiError>>(
 id: string, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetProblemReport<TData = Awaited<ReturnType<typeof getProblemReport>>, TError = ErrorType<ApiError>>(
 id: string, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getProblemReport>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetProblemReportQueryOptions(id,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListReportsUrl = (params?: ListReportsParams,) => {
  const normalizedParams = new URLSearchParams();

  Object.entries(params || {}).forEach(([key, value]) => {

    if (value !== undefined) {
      normalizedParams.append(key, value === null ? 'null' : String(value))
    }
  });

  const stringifiedParams = normalizedParams.toString();

  return stringifiedParams.length > 0 ? `/panel/api/v1/reports?${stringifiedParams}` : `/panel/api/v1/reports`
}

export const listReports = async (params?: ListReportsParams, options?: Parameters<typeof apiFetch>[1]): Promise<ReportList> => {

  return apiFetch<ReportList>(getListReportsUrl(params),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListReportsQueryKey = (params?: ListReportsParams,) => {
    return [
    `/panel/api/v1/reports`, ...(params ? [params] : [])
    ] as const;
    }


export const getListReportsQueryOptions = <TData = Awaited<ReturnType<typeof listReports>>, TError = ErrorType<ApiError>>(params?: ListReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListReportsQueryKey(params);



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listReports>>> = ({ signal }) => listReports(params, { signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListReportsQueryResult = NonNullable<Awaited<ReturnType<typeof listReports>>>
export type ListReportsQueryError = ErrorType<ApiError>


export function useListReports<TData = Awaited<ReturnType<typeof listReports>>, TError = ErrorType<ApiError>>(
 params: undefined |  ListReportsParams, options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listReports>>,
          TError,
          Awaited<ReturnType<typeof listReports>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListReports<TData = Awaited<ReturnType<typeof listReports>>, TError = ErrorType<ApiError>>(
 params?: ListReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listReports>>,
          TError,
          Awaited<ReturnType<typeof listReports>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListReports<TData = Awaited<ReturnType<typeof listReports>>, TError = ErrorType<ApiError>>(
 params?: ListReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListReports<TData = Awaited<ReturnType<typeof listReports>>, TError = ErrorType<ApiError>>(
 params?: ListReportsParams, options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listReports>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListReportsQueryOptions(params,options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getHandleReportUrl = () => {




  return `/panel/api/v1/reports/handle`
}

export const handleReport = async (handleReportRequest: HandleReportRequest,
    headers: HandleReportHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<HandleReportResult> => {

  return apiFetch<HandleReportResult>(getHandleReportUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(handleReportRequest)
  }
);}





export const getHandleReportMutationKey = () => ['handleReport'] as const;

export const getHandleReportMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof handleReport>>, TError,HandleReportMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof handleReport>>, TError,HandleReportMutationVariables, TContext> => {

const mutationKey = getHandleReportMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof handleReport>>, HandleReportMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  handleReport(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type HandleReportMutationResult = NonNullable<Awaited<ReturnType<typeof handleReport>>>
    export type HandleReportMutationBody = HandleReportRequest
    export type HandleReportMutationError = ErrorType<ApiError>
    export type HandleReportMutationVariables = {data: HandleReportRequest;headers: HandleReportHeaders}

    export const useHandleReport = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof handleReport>>, TError,HandleReportMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof handleReport>>,
        TError,
        HandleReportMutationVariables,
        TContext
      > => {
      return useMutation(getHandleReportMutationOptions(options), queryClient);
    }

export const getGetServerStatusUrl = () => {




  return `/panel/api/v1/server`
}

export const getServerStatus = async ( options?: Parameters<typeof apiFetch>[1]): Promise<ServerOverview> => {

  return apiFetch<ServerOverview>(getGetServerStatusUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetServerStatusQueryKey = () => {
    return [
    `/panel/api/v1/server`
    ] as const;
    }


export const getGetServerStatusQueryOptions = <TData = Awaited<ReturnType<typeof getServerStatus>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetServerStatusQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getServerStatus>>> = ({ signal }) => getServerStatus({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetServerStatusQueryResult = NonNullable<Awaited<ReturnType<typeof getServerStatus>>>
export type GetServerStatusQueryError = ErrorType<ApiError>


export function useGetServerStatus<TData = Awaited<ReturnType<typeof getServerStatus>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getServerStatus>>,
          TError,
          Awaited<ReturnType<typeof getServerStatus>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerStatus<TData = Awaited<ReturnType<typeof getServerStatus>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getServerStatus>>,
          TError,
          Awaited<ReturnType<typeof getServerStatus>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerStatus<TData = Awaited<ReturnType<typeof getServerStatus>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetServerStatus<TData = Awaited<ReturnType<typeof getServerStatus>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getServerStatus>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetServerStatusQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getStopServerUrl = () => {




  return `/panel/api/v1/server/stop`
}

export const stopServer = async (stopRequest: StopRequest,
    headers: StopServerHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<StopResponse> => {

  return apiFetch<StopResponse>(getStopServerUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(stopRequest)
  }
);}





export const getStopServerMutationKey = () => ['stopServer'] as const;

export const getStopServerMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof stopServer>>, TError,StopServerMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof stopServer>>, TError,StopServerMutationVariables, TContext> => {

const mutationKey = getStopServerMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof stopServer>>, StopServerMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  stopServer(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type StopServerMutationResult = NonNullable<Awaited<ReturnType<typeof stopServer>>>
    export type StopServerMutationBody = StopRequest
    export type StopServerMutationError = ErrorType<ApiError>
    export type StopServerMutationVariables = {data: StopRequest;headers: StopServerHeaders}

    export const useStopServer = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof stopServer>>, TError,StopServerMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof stopServer>>,
        TError,
        StopServerMutationVariables,
        TContext
      > => {
      return useMutation(getStopServerMutationOptions(options), queryClient);
    }

export const getGetStaffSessionUrl = () => {




  return `/panel/api/v1/session`
}

export const getStaffSession = async ( options?: Parameters<typeof apiFetch>[1]): Promise<StaffSession> => {

  return apiFetch<StaffSession>(getGetStaffSessionUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getGetStaffSessionQueryKey = () => {
    return [
    `/panel/api/v1/session`
    ] as const;
    }


export const getGetStaffSessionQueryOptions = <TData = Awaited<ReturnType<typeof getStaffSession>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getGetStaffSessionQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof getStaffSession>>> = ({ signal }) => getStaffSession({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type GetStaffSessionQueryResult = NonNullable<Awaited<ReturnType<typeof getStaffSession>>>
export type GetStaffSessionQueryError = ErrorType<ApiError>


export function useGetStaffSession<TData = Awaited<ReturnType<typeof getStaffSession>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getStaffSession>>,
          TError,
          Awaited<ReturnType<typeof getStaffSession>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetStaffSession<TData = Awaited<ReturnType<typeof getStaffSession>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getStaffSession>>,
          TError,
          Awaited<ReturnType<typeof getStaffSession>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetStaffSession<TData = Awaited<ReturnType<typeof getStaffSession>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useGetStaffSession<TData = Awaited<ReturnType<typeof getStaffSession>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof getStaffSession>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getGetStaffSessionQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getListSettingsUrl = () => {




  return `/panel/api/v1/settings`
}

export const listSettings = async ( options?: Parameters<typeof apiFetch>[1]): Promise<SettingsList> => {

  return apiFetch<SettingsList>(getListSettingsUrl(),
  {
    ...options,
    method: 'GET'


  }
);}





export const getListSettingsQueryKey = () => {
    return [
    `/panel/api/v1/settings`
    ] as const;
    }


export const getListSettingsQueryOptions = <TData = Awaited<ReturnType<typeof listSettings>>, TError = ErrorType<ApiError>>( options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
) => {

const {query: queryOptions, request: requestOptions} = options ?? {};

  const queryKey =  queryOptions?.queryKey ?? getListSettingsQueryKey();



    const queryFn: QueryFunction<Awaited<ReturnType<typeof listSettings>>> = ({ signal }) => listSettings({ signal, ...requestOptions });





   return  { queryKey, queryFn, ...queryOptions} as UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData> & { queryKey: DataTag<QueryKey, TData, TError> }
}

export type ListSettingsQueryResult = NonNullable<Awaited<ReturnType<typeof listSettings>>>
export type ListSettingsQueryError = ErrorType<ApiError>


export function useListSettings<TData = Awaited<ReturnType<typeof listSettings>>, TError = ErrorType<ApiError>>(
  options: { query:Partial<UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData>> & Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof listSettings>>,
          TError,
          Awaited<ReturnType<typeof listSettings>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListSettings<TData = Awaited<ReturnType<typeof listSettings>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData>> & Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof listSettings>>,
          TError,
          Awaited<ReturnType<typeof listSettings>>
        > , 'initialData'
      >, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListSettings<TData = Awaited<ReturnType<typeof listSettings>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
  ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> }
export function useListSettings<TData = Awaited<ReturnType<typeof listSettings>>, TError = ErrorType<ApiError>>(
  options?: { query?:Partial<UseQueryOptions<Awaited<ReturnType<typeof listSettings>>, TError, TData>>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient
 ):  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {

  const queryOptions = getListSettingsQueryOptions(options)

  const query = useQuery(queryOptions, queryClient) as  UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

  return withQueryKey(query, queryOptions.queryKey);
}







export const getChangeSettingsUrl = () => {




  return `/panel/api/v1/settings`
}

export const changeSettings = async (settingsRequest: SettingsRequest,
    headers: ChangeSettingsHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<SettingsResult> => {

  return apiFetch<SettingsResult>(getChangeSettingsUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(settingsRequest)
  }
);}





export const getChangeSettingsMutationKey = () => ['changeSettings'] as const;

export const getChangeSettingsMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof changeSettings>>, TError,ChangeSettingsMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof changeSettings>>, TError,ChangeSettingsMutationVariables, TContext> => {

const mutationKey = getChangeSettingsMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof changeSettings>>, ChangeSettingsMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  changeSettings(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type ChangeSettingsMutationResult = NonNullable<Awaited<ReturnType<typeof changeSettings>>>
    export type ChangeSettingsMutationBody = SettingsRequest
    export type ChangeSettingsMutationError = ErrorType<ApiError>
    export type ChangeSettingsMutationVariables = {data: SettingsRequest;headers: ChangeSettingsHeaders}

    export const useChangeSettings = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof changeSettings>>, TError,ChangeSettingsMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof changeSettings>>,
        TError,
        ChangeSettingsMutationVariables,
        TContext
      > => {
      return useMutation(getChangeSettingsMutationOptions(options), queryClient);
    }

export const getSignInStaffUrl = () => {




  return `/panel/api/v1/sign-in`
}

export const signInStaff = async (signInRequest: SignInRequest,
    headers: SignInStaffHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<SignInOutcome> => {

  return apiFetch<SignInOutcome>(getSignInStaffUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(signInRequest)
  }
);}





export const getSignInStaffMutationKey = () => ['signInStaff'] as const;

export const getSignInStaffMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signInStaff>>, TError,SignInStaffMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof signInStaff>>, TError,SignInStaffMutationVariables, TContext> => {

const mutationKey = getSignInStaffMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof signInStaff>>, SignInStaffMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  signInStaff(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type SignInStaffMutationResult = NonNullable<Awaited<ReturnType<typeof signInStaff>>>
    export type SignInStaffMutationBody = SignInRequest
    export type SignInStaffMutationError = ErrorType<ApiError>
    export type SignInStaffMutationVariables = {data: SignInRequest;headers: SignInStaffHeaders}

    export const useSignInStaff = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signInStaff>>, TError,SignInStaffMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof signInStaff>>,
        TError,
        SignInStaffMutationVariables,
        TContext
      > => {
      return useMutation(getSignInStaffMutationOptions(options), queryClient);
    }

export const getVerifyStaffCodeUrl = () => {




  return `/panel/api/v1/sign-in/code`
}

export const verifyStaffCode = async (signInCodeRequest: SignInCodeRequest,
    headers: VerifyStaffCodeHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<StaffSession> => {

  return apiFetch<StaffSession>(getVerifyStaffCodeUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(signInCodeRequest)
  }
);}





export const getVerifyStaffCodeMutationKey = () => ['verifyStaffCode'] as const;

export const getVerifyStaffCodeMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof verifyStaffCode>>, TError,VerifyStaffCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof verifyStaffCode>>, TError,VerifyStaffCodeMutationVariables, TContext> => {

const mutationKey = getVerifyStaffCodeMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof verifyStaffCode>>, VerifyStaffCodeMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  verifyStaffCode(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type VerifyStaffCodeMutationResult = NonNullable<Awaited<ReturnType<typeof verifyStaffCode>>>
    export type VerifyStaffCodeMutationBody = SignInCodeRequest
    export type VerifyStaffCodeMutationError = ErrorType<ApiError>
    export type VerifyStaffCodeMutationVariables = {data: SignInCodeRequest;headers: VerifyStaffCodeHeaders}

    export const useVerifyStaffCode = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof verifyStaffCode>>, TError,VerifyStaffCodeMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof verifyStaffCode>>,
        TError,
        VerifyStaffCodeMutationVariables,
        TContext
      > => {
      return useMutation(getVerifyStaffCodeMutationOptions(options), queryClient);
    }

export const getSignOutStaffUrl = () => {




  return `/panel/api/v1/sign-out`
}

export const signOutStaff = async (headers: SignOutStaffHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<SignedOut> => {

  return apiFetch<SignedOut>(getSignOutStaffUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders(headers, options?.headers)

  }
);}





export const getSignOutStaffMutationKey = () => ['signOutStaff'] as const;

export const getSignOutStaffMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signOutStaff>>, TError,SignOutStaffMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof signOutStaff>>, TError,SignOutStaffMutationVariables, TContext> => {

const mutationKey = getSignOutStaffMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof signOutStaff>>, SignOutStaffMutationVariables> = (props) => {
          const {headers} = props ?? {};

          return  signOutStaff(headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type SignOutStaffMutationResult = NonNullable<Awaited<ReturnType<typeof signOutStaff>>>

    export type SignOutStaffMutationError = ErrorType<ApiError>
    export type SignOutStaffMutationVariables = {headers: SignOutStaffHeaders}

    export const useSignOutStaff = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signOutStaff>>, TError,SignOutStaffMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof signOutStaff>>,
        TError,
        SignOutStaffMutationVariables,
        TContext
      > => {
      return useMutation(getSignOutStaffMutationOptions(options), queryClient);
    }

export const getSignOutEverywhereUrl = () => {




  return `/panel/api/v1/sign-out-everywhere`
}

export const signOutEverywhere = async (emptyRequest: EmptyRequest,
    headers: SignOutEverywhereHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<SignedOut> => {

  return apiFetch<SignedOut>(getSignOutEverywhereUrl(),
  {
    ...options,
    method: 'POST',
    headers: getHeaders({ 'Content-Type': 'application/json' }, headers, options?.headers),
    body: JSON.stringify(emptyRequest)
  }
);}





export const getSignOutEverywhereMutationKey = () => ['signOutEverywhere'] as const;

export const getSignOutEverywhereMutationOptions = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signOutEverywhere>>, TError,SignOutEverywhereMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
): UseMutationOptions<Awaited<ReturnType<typeof signOutEverywhere>>, TError,SignOutEverywhereMutationVariables, TContext> => {

const mutationKey = getSignOutEverywhereMutationKey();
const {mutation: mutationOptions, request: requestOptions} = options ?
      options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey ?
      options
      : {...options, mutation: {...options.mutation, mutationKey}}
      : {mutation: { mutationKey, }, request: undefined};




      const mutationFn: MutationFunction<Awaited<ReturnType<typeof signOutEverywhere>>, SignOutEverywhereMutationVariables> = (props) => {
          const {data,headers} = props ?? {};

          return  signOutEverywhere(data,headers,requestOptions)
        }






  return  { mutationFn, ...mutationOptions }}

    export type SignOutEverywhereMutationResult = NonNullable<Awaited<ReturnType<typeof signOutEverywhere>>>
    export type SignOutEverywhereMutationBody = EmptyRequest
    export type SignOutEverywhereMutationError = ErrorType<ApiError>
    export type SignOutEverywhereMutationVariables = {data: EmptyRequest;headers: SignOutEverywhereHeaders}

    export const useSignOutEverywhere = <TError = ErrorType<ApiError>,
    TContext = unknown>(options?: { mutation?:UseMutationOptions<Awaited<ReturnType<typeof signOutEverywhere>>, TError,SignOutEverywhereMutationVariables, TContext>, request?: SecondParameter<typeof apiFetch>}
 , queryClient?: QueryClient): UseMutationResult<
        Awaited<ReturnType<typeof signOutEverywhere>>,
        TError,
        SignOutEverywhereMutationVariables,
        TContext
      > => {
      return useMutation(getSignOutEverywhereMutationOptions(options), queryClient);
    }
