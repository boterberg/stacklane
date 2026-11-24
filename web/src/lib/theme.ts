import { useSyncExternalStore } from 'react'

export type Theme = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'stacklane-theme'
const darkQuery = window.matchMedia('(prefers-color-scheme: dark)')
const listeners = new Set<() => void>()

let current: Theme = readStoredTheme()

function readStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    return stored === 'light' || stored === 'dark' ? stored : 'system'
  } catch {
    return 'system'
  }
}

function render() {
  const dark = current === 'dark' || (current === 'system' && darkQuery.matches)
  document.documentElement.classList.toggle('dark', dark)
}

export function getTheme() {
  return current
}

export function applyTheme(theme: Theme) {
  current = theme
  render()
  try {
    localStorage.setItem(STORAGE_KEY, theme)
  } catch {}
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useTheme() {
  return useSyncExternalStore(subscribe, getTheme)
}

render()
darkQuery.addEventListener('change', render)
