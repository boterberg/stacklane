import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type FloatingIp = {
  id: string
  address: string | null
  status: 'pending' | 'provisioning' | 'active' | 'suspended' | 'failed'
  integration_id: string | null
  server_id: string | null
  server_name: string | null
  subscription_code: string | null
  cancelled: boolean
  guest_setup: boolean
  last_error: string | null
  created_at: string
}

export type FloatingIpOptions = {
  products: { product_id: string; name: string; monthly_price_cents: number }[]
  servers: { server_id: string; server_name: string; integration_id: string }[]
}

export function useFloatingIps() {
  return useQuery({
    queryKey: ['vps', 'floating-ips'],
    queryFn: () => api<{ floating_ips: FloatingIp[]; used: number; limit: number }>('/vps/floating-ips'),
  })
}

export function useFloatingIpOptions(enabled: boolean) {
  return useQuery({ queryKey: ['vps', 'floating-ips', 'options'], queryFn: () => api<FloatingIpOptions>('/vps/floating-ips/options'), enabled })
}

export function useOrderFloatingIps() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { product_id: string; quantity: number; billing_interval: string }) =>
      api<{ invoice_id: number }>('/vps/floating-ips/order', { method: 'POST', body: input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['vps'] })
      queryClient.invalidateQueries({ queryKey: ['billing'] })
    },
  })
}

export function useFloatingIpAction() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, action, serverId }: { id: string; action: 'attach' | 'detach'; serverId?: string }) =>
      api<void>(`/vps/floating-ips/${id}/${action}`, { method: 'POST', body: action === 'attach' ? { server_id: serverId } : undefined }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'floating-ips'] }),
  })
}
