import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type Network = {
  id: string
  name: string
  cidr: string
  integration_id: string
  platform: string
  restart_needed: boolean
  servers: number
  created_at: string
}

export type NetworkOverview = {
  networks: Network[]
  platforms: { integration_id: string; name: string }[]
  limit: number
  suggested_name: string
  suggested_cidr: string
}

export type NetworkDetail = Network & {
  members: { server_id: string; server_name: string; address: string }[]
  candidates: { server_id: string; server_name: string }[]
}

export function useNetworks() {
  return useQuery({ queryKey: ['vps', 'networks'], queryFn: () => api<NetworkOverview>('/vps/networks') })
}

export function useNetwork(id: string) {
  return useQuery({ queryKey: ['vps', 'networks', id], queryFn: () => api<NetworkDetail>(`/vps/networks/${id}`), retry: false })
}

export function useCreateNetwork() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { name: string; cidr: string; integration_id: string }) => api<Network>('/vps/networks', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'networks'] }),
  })
}

export function useRenameNetwork(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (name: string) => api<void>(`/vps/networks/${id}`, { method: 'PUT', body: { name } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'networks'] }),
  })
}

export function useDeleteNetwork() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/vps/networks/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'networks'] }),
  })
}

export function useNetworkServer(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ serverId, attach }: { serverId: string; attach: boolean }) =>
      attach
        ? api<void>(`/vps/networks/${id}/servers`, { method: 'POST', body: { server_id: serverId } })
        : api<void>(`/vps/networks/${id}/servers/${serverId}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'networks'] }),
  })
}
