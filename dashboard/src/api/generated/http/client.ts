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
  AccountCodeResponse,
  AccountDetail,
  AccountEditRequest,
  AccountEditResult,
  AccountList,
  ApiError,
  BpCodeList,
  CountryList,
  CreateBpCodeHeaders,
  CreateBpCodeRequest,
  CreateBpCodeResult,
  CreateInviteHeaders,
  CreateInviteRequest,
  CreateInviteResult,
  EditAccountHeaders,
  GiveBpByIdRequest,
  GiveBpHeaders,
  GiveBpResult,
  InviteList,
  IssueAccountCodeHeaders,
  IssueAccountCodeRequest,
  KeyAccountRequest,
  KeyAccountResult,
  ManageAccountKeyHeaders,
  RemoveBpCodeHeaders,
  RemoveBpCodeRequest,
  RemoveBpCodeResult,
  SignInRequest,
  SignInStaffHeaders,
  SignOutStaffHeaders,
  SignedOut,
  StaffSession
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







export const getSignInStaffUrl = () => {




  return `/panel/api/v1/sign-in`
}

export const signInStaff = async (signInRequest: SignInRequest,
    headers: SignInStaffHeaders, options?: Parameters<typeof apiFetch>[1]): Promise<StaffSession> => {

  return apiFetch<StaffSession>(getSignInStaffUrl(),
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
