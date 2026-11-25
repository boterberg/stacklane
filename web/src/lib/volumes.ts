import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type Volume = {
  id: string
  name: string
  status: 'pending' | 'provisioning' | 'active' | 'suspended' | 'failed'
  size_gb: number | null
  server_id: string | null
  server_name: string | null
  subscription_code: string | null
  cancelled: boolean
  last_error: string | null
  created_at: string
}

export type VolumeOption = { server_id: string; server_name: string; product_id: string; price_per_gb_cents: number }

export function useVolumes() {
  return useQuery({ queryKey: ['vps', 'volumes'], queryFn: () => api<{ volumes: Volume[]; used: number; limit: number }>('/vps/volumes') })
}

export function useVolumeOptions(enabled: boolean) {
  return useQuery({ queryKey: ['vps', 'volumes', 'options'], queryFn: () => api<VolumeOption[]>('/vps/volumes/options'), enabled })
}

export function useOrderVolume() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { product_id: string; server_id: string; name: string; size_gb: number; billing_interval: string }) =>
      api<{ invoice_id: number }>('/orders', { method: 'POST', body: { ...input, image: null, ssh_key_ids: [] } }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['vps'] })
      queryClient.invalidateQueries({ queryKey: ['billing'] })
    },
  })
}

export function useVolumeAction() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, action, serverId }: { id: string; action: 'attach' | 'detach'; serverId?: string }) =>
      api<void>(`/vps/volumes/${id}/${action}`, { method: 'POST', body: action === 'attach' ? { server_id: serverId } : undefined }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'volumes'] }),
  })
}
