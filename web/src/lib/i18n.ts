import { useSyncExternalStore } from 'react'

import { en } from '../locales/en'

export type Locale = 'nl' | 'en'

export const locales: Array<{ code: Locale; name: string }> = [
  { code: 'nl', name: 'Nederlands' },
  { code: 'en', name: 'English' },
]

const STORAGE_KEY = 'stacklane-locale'
const listeners = new Set<() => void>()

function initialLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'nl' || stored === 'en') return stored
  } catch {}
  return navigator.language.toLowerCase().startsWith('en') ? 'en' : 'nl'
}

let current: Locale = initialLocale()
document.documentElement.lang = current

export function getLocale() {
  return current
}

export function isLocale(value: string): value is Locale {
  return value === 'nl' || value === 'en'
}

export function setLocale(locale: Locale) {
  if (locale === current) return
  current = locale
  document.documentElement.lang = locale
  try {
    localStorage.setItem(STORAGE_KEY, locale)
  } catch {}
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useLocale() {
  return useSyncExternalStore(subscribe, getLocale)
}

export function intlLocale() {
  return current === 'en' ? 'en-GB' : 'nl-NL'
}

export function t(text: string, values?: Record<string, string | number>) {
  const translated = current === 'en' ? (en[text] ?? text) : text
  if (!values) return translated
  return translated.replace(/\{(\w+)\}/g, (match, key: string) => (key in values ? String(values[key]) : match))
}
