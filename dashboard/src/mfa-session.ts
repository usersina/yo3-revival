import { useQueryClient } from '@tanstack/react-query'
import { getGetStaffSessionQueryKey } from './api/generated/http/client'
import type { StaffSession } from './api/generated/http/models'
import { setCsrfToken } from './api/transport'

/** Raised when the server answers `reauthentication_required`: the step-up dialog opens. */
export const STEP_UP_EVENT = 'yo3:step-up'

/** Makes a session the dashboard's current one (after sign-in, enrollment or a step-up). */
export function useAcceptSession() {
  const client = useQueryClient()
  return (session: StaffSession, reset = false) => {
    setCsrfToken(session.csrf_token)
    client.setQueryData(getGetStaffSessionQueryKey(), session)
    if (reset) {
      void client.resetQueries({
        predicate: (query) => query.queryKey[0] !== getGetStaffSessionQueryKey()[0],
      })
    }
  }
}
