import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api, ApiError } from './api'
import { applyTheme, type Theme } from './theme'

export type Profile = {
  id: string
  email: string
  first_name: string
  last_name: string
  company_name: string | null
  is_business: boolean
  country: string
  theme: Theme
  locale: string
  timezone: string
}

export const meQueryKey = ['me'] as const

async function fetchMe(): Promise<Profile | null> {
  try {
    const profile = await api<Profile>('/me')
    applyTheme(profile.theme)
    return profile
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      return null
    }
    throw error
  }
}

export function useMe() {
  return useQuery({ queryKey: meQueryKey, queryFn: fetchMe, staleTime: 60_000, retry: false })
}

export function displayName(profile: Profile) {
  return [profile.first_name, profile.last_name].filter(Boolean).join(' ') || profile.email
}

export function initials(profile: Profile) {
  const letters = [profile.first_name, profile.last_name].filter(Boolean).map((part) => part[0])
  return (letters.join('') || profile.email[0]).toUpperCase()
}

export function useAuthMutation<TInput>(path: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: TInput) => api<Profile>(path, { method: 'POST', body: input }),
    onSuccess: (profile) => {
      applyTheme(profile.theme)
      queryClient.setQueryData(meQueryKey, profile)
    },
  })
}

export function useLogout() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => api<void>('/auth/logout', { method: 'POST' }),
    onSettled: () => {
      queryClient.clear()
      queryClient.setQueryData(meQueryKey, null)
    },
  })
}
