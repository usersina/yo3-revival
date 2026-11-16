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
  AccountList,
  ApiError,
  GiveBpByIdRequest,
  GiveBpHeaders,
  GiveBpResult,
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
