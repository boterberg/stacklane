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

export type DomainStatus = 'pending' | 'provisioning' | 'active' | 'suspended' | 'failed'

export type Domain = {
  id: string
  name: string
  status: DomainStatus
  details: { expires_on?: string | null; nameservers?: string[]; operation?: 'register' | 'transfer'; whois_profile_id?: string; registrar_status?: string }
  last_error: string | null
  subscription_code: string | null
  expires_at: string | null
  cancelled: boolean
  created_at: string
}

export type Registry = { status: string; expires_on: string | null; nameservers: string[]; locked: boolean }

export type SearchResult = {
  domain: string
  extension: string
  available: boolean
  premium: boolean
  register_cents: number
  renew_cents: number
  transfer_cents: number
}

export function useDomains() {
  return useQuery({
    queryKey: ['domains'],
    queryFn: () => api<Domain[]>('/domains'),
    refetchInterval: (query) => (query.state.data?.some((domain) => domain.status === 'pending' || domain.status === 'provisioning') ? 5000 : false),
  })
}

export function useDomain(id: string) {
  return useQuery({
    queryKey: ['domains', id],
    queryFn: () => api<Domain & { registry: Registry | null }>(`/domains/${id}`),
    refetchInterval: (query) => (query.state.data?.status === 'active' ? false : 5000),
  })
}

export function useDomainSearch(query: string) {
  return useQuery({
    queryKey: ['domains', 'search', query],
    queryFn: () => api<SearchResult[]>(`/domains/search?query=${encodeURIComponent(query)}`),
    enabled: query.trim().length > 0,
    retry: false,
    staleTime: 60_000,
  })
}

export function useOrderDomain() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { domain: string; whois_profile_id: string; transfer: boolean; auth_code?: string }) =>
      api<{ invoice_id: number; service_id: string }>('/domains/orders', { method: 'POST', body: input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['domains'] })
      queryClient.invalidateQueries({ queryKey: ['billing'] })
    },
  })
}

export function useSetNameservers(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (nameservers: string[]) => api<void>(`/domains/${id}/nameservers`, { method: 'PUT', body: { nameservers } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['domains', id] }),
  })
}

export function useAuthCode(id: string) {
  return useMutation({ mutationFn: () => api<{ auth_code: string }>(`/domains/${id}/auth-code`, { method: 'POST' }) })
}

export function useSetLock(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (locked: boolean) => api<void>(`/domains/${id}/lock`, { method: 'PUT', body: { locked } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['domains', id] }),
  })
}
