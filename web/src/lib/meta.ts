import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { applyPreferences, meQueryKey, syncPreferences, type Profile } from './auth'
import { applyTheme } from './brandTheme'
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
  accent_dark: string | null
  neutral: string | null
  radius: string | null
  css: string | null
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

export function applyBranding(branding: Branding) {
  applyTheme(branding)
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
