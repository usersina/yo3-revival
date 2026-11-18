import { useSyncExternalStore } from 'react'

export type ThemePreference = 'system' | 'light' | 'dark'
const storageKey = 'yo3:theme'
const changeEvent = 'yo3-theme-change'
function readPreference(): ThemePreference {
  try {
    const saved = localStorage.getItem(storageKey)
    return saved === 'light' || saved === 'dark' ? saved : 'system'
  } catch {
    return 'system'
  }
}
const deviceTheme = () =>
  window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
let preference = readPreference()
let resolved = preference === 'system' ? deviceTheme() : preference
function apply() {
  resolved = preference === 'system' ? deviceTheme() : preference
  document.documentElement.dataset.theme = resolved
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute('content', resolved === 'dark' ? '#0c121c' : '#f5f7fa')
  window.dispatchEvent(new Event(changeEvent))
}
// Set the palette before React renders the first controls.
export function initializeTheme() {
  preference = readPreference()
  apply()
}
function setPreference(next: ThemePreference) {
  preference = next
  try {
    localStorage.setItem(storageKey, next)
  } catch {
    // Keep the toggle usable even when browser storage is unavailable.
  }
  apply()
}
function subscribe(listener: () => void) {
  const media = window.matchMedia?.('(prefers-color-scheme: dark)')
  const onDeviceChange = () => {
    if (preference === 'system') apply()
  }
  const onStorage = (event: StorageEvent) => {
    if (event.key === storageKey || event.key === null) initializeTheme()
  }
  window.addEventListener(changeEvent, listener)
  window.addEventListener('storage', onStorage)
  media?.addEventListener('change', onDeviceChange)
  return () => {
    window.removeEventListener(changeEvent, listener)
    window.removeEventListener('storage', onStorage)
    media?.removeEventListener('change', onDeviceChange)
  }
}
const snapshot = () => `${preference}:${resolved}`
export function useTheme() {
  const state = useSyncExternalStore(subscribe, snapshot)
  const [selected, theme] = state.split(':') as [ThemePreference, 'light' | 'dark']
  return { preference: selected, theme, setPreference }
}
