import { createContext } from 'react'

export const FieldContext = createContext<{ id: string; hintId?: string }>({ id: '' })
