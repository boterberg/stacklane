import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type WhoisProfile = {
  id: string
  first_name: string
  last_name: string
  company: string | null
  email: string
  phone: string
  street: string
  house_number: string
  zipcode: string
  city: string
  country: string
  domains: number
  created_at: string
}

export type WhoisInput = Omit<WhoisProfile, 'id' | 'domains' | 'created_at'>

export function useWhoisProfiles() {
  return useQuery({ queryKey: ['whois-profiles'], queryFn: () => api<WhoisProfile[]>('/whois-profiles') })
}

export function useSaveWhoisProfile() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...input }: WhoisInput & { id?: string }) =>
      api<WhoisProfile>(id ? `/whois-profiles/${id}` : '/whois-profiles', { method: id ? 'PUT' : 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['whois-profiles'] }),
  })
}

export function useDeleteWhoisProfile() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/whois-profiles/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['whois-profiles'] }),
  })
}

export function holderName(profile: Pick<WhoisProfile, 'first_name' | 'last_name' | 'company'>) {
  return profile.company ? `${profile.company} (${profile.first_name} ${profile.last_name})` : `${profile.first_name} ${profile.last_name}`
}

export function displayPhone(phone: string) {
  return phone.replace('.', ' ')
}
