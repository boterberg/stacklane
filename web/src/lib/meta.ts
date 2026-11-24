import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { applyPreferences, meQueryKey, syncPreferences, type Profile } from './auth'

export type Company = {
  name: string
  address: string | null
  email: string | null
  website: string | null
  coc_number: string | null
  vat_number: string | null
}

export type Meta = {
  version: string
  demo: boolean
  next_reset_at: string | null
  company: Company | null
}

export type DemoRole = 'customer' | 'admin'

export function useMeta() {
  return useQuery({ queryKey: ['meta'], queryFn: () => api<Meta>('/meta'), staleTime: 5 * 60_000 })
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
