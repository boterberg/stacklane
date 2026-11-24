import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { meQueryKey, type Profile } from './auth'
import { applyTheme } from './theme'

export type Meta = {
  version: string
  demo: boolean
  next_reset_at: string | null
}

export type DemoRole = 'customer' | 'admin'

export function useMeta() {
  return useQuery({ queryKey: ['meta'], queryFn: () => api<Meta>('/meta'), staleTime: 5 * 60_000 })
}

export function useDemoLogin() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (role: DemoRole) => api<Profile>('/demo/login', { method: 'POST', body: { role } }),
    onSuccess: (profile) => {
      queryClient.clear()
      applyTheme(profile.theme)
      queryClient.setQueryData(meQueryKey, profile)
    },
  })
}
