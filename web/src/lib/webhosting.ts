import { useMutation, useQuery } from '@tanstack/react-query'

import { api } from './api'
import type { VpsStatus as ServiceStatus } from './vps'

export type Website = {
  id: string
  name: string
  status: ServiceStatus
  product_name: string | null
  details: { username?: string; plan?: string; panel?: string; panel_url?: string }
  last_error: string | null
  subscription_code: string | null
  expires_at: string | null
  created_at: string
}

const busy = (status: ServiceStatus) => status === 'pending' || status === 'provisioning'

export function useWebsites() {
  return useQuery({
    queryKey: ['webhosting'],
    queryFn: () => api<Website[]>('/webhosting'),
    refetchInterval: (query) => (query.state.data?.some((website) => busy(website.status)) ? 5000 : false),
  })
}

export function useWebsite(id: string) {
  return useQuery({
    queryKey: ['webhosting', id],
    queryFn: () => api<Website>(`/webhosting/${id}`),
    refetchInterval: (query) => (query.state.data && busy(query.state.data.status) ? 5000 : false),
  })
}

export function usePanelLogin(id: string) {
  return useMutation({ mutationFn: () => api<{ url: string }>(`/webhosting/${id}/panel`, { method: 'POST' }) })
}
