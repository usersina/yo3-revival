import { createContext, useContext } from 'react'
import type { DemoState, Role, Settings } from './data'

export interface Notice {
  message: string
  tone: 'success' | 'error' | 'info'
}

export interface Preview {
  data: DemoState
  draft: Settings
  setDraft: (draft: Settings) => void
  generation: number
  role: Role
  setRole: (role: Role) => void
  notice: Notice | null
  notify: (message: string, tone?: Notice['tone']) => void
  dismiss: () => void
  change: (update: (previous: DemoState) => DemoState, action: string, target?: string) => void
  reset: () => void
}

// Keep the context identity outside the component module so SWC Fast Refresh can replace the provider safely.
export const PreviewContext = createContext<Preview | null>(null)

export function usePreview() {
  const value = useContext(PreviewContext)
  if (!value) throw new Error('PreviewProvider is missing')
  return value
}
