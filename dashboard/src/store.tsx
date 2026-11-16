import { useState } from 'react'
import type { ReactNode } from 'react'
import { createDemoState, id } from './data'
import type { DemoState, Role } from './data'
import { PreviewContext } from './preview'
import type { Notice } from './preview'

export function PreviewProvider({ children }: { children: ReactNode }) {
  const [data, setData] = useState(createDemoState)
  const [draft, setDraft] = useState(data.settings)
  const [generation, setGeneration] = useState(0)
  const [role, setRole] = useState<Role>('admin')
  const [notice, setNotice] = useState<Notice | null>(null)
  function change(update: (previous: DemoState) => DemoState, action: string, target = 'Server') {
    const entry = {
      id: id(),
      at: new Date().toISOString(),
      actor: role === 'admin' ? 'TestAdmin' : 'TestModerator',
      action,
      target,
      source: 'Dashboard',
    }
    setData((previous) => {
      const updated = update(previous)
      return { ...updated, audit: [entry, ...updated.audit] }
    })
  }
  function reset() {
    const fresh = createDemoState()
    setData(fresh)
    setDraft(fresh.settings)
    setGeneration((n) => n + 1)
    setRole('admin')
    setNotice({ message: 'Preview reset. All sample data restored.', tone: 'info' })
  }
  return (
    <PreviewContext.Provider
      value={{
        data,
        draft,
        setDraft,
        generation,
        role,
        setRole,
        notice,
        notify: (message, tone = 'success') => setNotice({ message, tone }),
        dismiss: () => setNotice(null),
        change,
        reset,
      }}
    >
      {children}
    </PreviewContext.Provider>
  )
}
