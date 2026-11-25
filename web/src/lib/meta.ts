import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { applyPreferences, meQueryKey, syncPreferences, type Profile } from './auth'
import { setCurrency } from './format'

export type Company = {
  name: string
  address: string | null
  email: string | null
  website: string | null
  coc_number: string | null
  vat_number: string | null
}

export type Branding = {
  name: string | null
  accent: string | null
  logo: number | null
  logo_dark: number | null
  favicon: number | null
}

export type Meta = {
  demo: boolean
  next_reset_at: string | null
  company: Company | null
  currency: string
  branding: Branding
}

function contrast(hex: string) {
  const [r, g, b] = [1, 3, 5].map((start) => {
    const channel = parseInt(hex.slice(start, start + 2), 16) / 255
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? '#18181b' : '#ffffff'
}

export function applyBranding(branding: Branding) {
  const root = document.documentElement
  if (branding.accent) {
    root.dataset.accent = ''
    root.style.setProperty('--accent', branding.accent)
    root.style.setProperty('--accent-contrast', contrast(branding.accent))
  } else {
    delete root.dataset.accent
    root.style.removeProperty('--accent')
    root.style.removeProperty('--accent-contrast')
  }
  document.title = branding.name ?? 'Stacklane'
  const icons = document.querySelectorAll<HTMLLinkElement>('link[rel="icon"]')
  icons.forEach((icon) => {
    if (!icon.dataset.original) icon.dataset.original = icon.href
    icon.href = branding.favicon ? `/api/branding/favicon?v=${branding.favicon}` : icon.dataset.original
  })
}

export type DemoRole = 'customer' | 'admin'

export function useMeta() {
  return useQuery({
    queryKey: ['meta'],
    queryFn: async () => {
      const meta = await api<Meta>('/meta')
      setCurrency(meta.currency)
      applyBranding(meta.branding)
      return meta
    },
    staleTime: 5 * 60_000,
  })
}

export function useDemoLogin() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (role: DemoRole) => syncPreferences(await api<Profile>('/demo/login', { method: 'POST', body: { role } })),
    onSuccess: (profile) => {
      queryClient.clear()
      applyPreferences(profile)
      queryClient.setQueryData(meQueryKey, profile)
    },
  })
}
