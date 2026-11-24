import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type VpsStatus = 'pending' | 'provisioning' | 'active' | 'suspended' | 'failed'

export type VpsDetails = {
  hostname?: string
  ipv4?: string
  image?: string
  cores?: number
  memory_mb?: number
  disk_gb?: number
}

export type VpsServer = {
  id: string
  name: string
  status: VpsStatus
  details: VpsDetails
  last_error: string | null
  product_name: string | null
  subscription_code: string | null
  expires_at: string | null
  created_at: string
}

export type LiveStatus = { state: string; cpu: number | null; cpus: number; memory: number | null; max_memory: number; uptime: number | null }

export type Snapshot = { name: string; description: string; created_at: number | null }

export type FirewallRule = { id: string; type: 'in' | 'out'; action: string; proto?: string | null; dport?: string | null; source?: string | null; comment?: string | null }

export type SshKey = { id: string; name: string; fingerprint: string; created_at: string }

export type ShopProduct = {
  id: string
  name: string
  description: string | null
  specs: { cores?: number; memory_mb?: number; disk_gb?: number }
  monthly_price_cents: number
  images: Array<{ id: number; name: string }>
}

export function useVpsServers() {
  return useQuery({
    queryKey: ['vps', 'servers'],
    queryFn: () => api<VpsServer[]>('/vps/servers'),
    placeholderData: keepPreviousData,
    refetchInterval: (query) => (query.state.data?.some((server) => server.status === 'pending' || server.status === 'provisioning') ? 5000 : false),
  })
}

export function useVpsServer(id: string) {
  return useQuery({
    queryKey: ['vps', 'server', id],
    queryFn: () => api<VpsServer & { live: LiveStatus | null }>(`/vps/servers/${id}`),
    refetchInterval: (query) => (query.state.data?.status === 'active' ? 10_000 : 5000),
  })
}

export function usePower(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (action: 'start' | 'shutdown' | 'stop' | 'reboot') => api<void>(`/vps/servers/${id}/power`, { method: 'POST', body: { action } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'server', id] }),
  })
}

export function useRootPassword(id: string) {
  return useMutation({ mutationFn: () => api<{ password: string | null }>(`/vps/servers/${id}/password`) })
}

export function useSnapshots(id: string, enabled: boolean) {
  return useQuery({ queryKey: ['vps', 'snapshots', id], queryFn: () => api<Snapshot[]>(`/vps/servers/${id}/snapshots`), enabled })
}

function useServerMutation<TInput>(_id: string, key: string, request: (input: TInput) => Promise<void>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: request,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', key] }),
  })
}

export function useCreateSnapshot(id: string) {
  return useServerMutation(id, 'snapshots', (description: string) => api<void>(`/vps/servers/${id}/snapshots`, { method: 'POST', body: { description } }))
}

export function useRollbackSnapshot(id: string) {
  return useServerMutation(id, 'snapshots', (name: string) => api<void>(`/vps/servers/${id}/snapshots/${name}/rollback`, { method: 'POST' }))
}

export function useDeleteSnapshot(id: string) {
  return useServerMutation(id, 'snapshots', (name: string) => api<void>(`/vps/servers/${id}/snapshots/${name}`, { method: 'DELETE' }))
}

export function useFirewall(id: string, enabled: boolean) {
  return useQuery({ queryKey: ['vps', 'firewall', id], queryFn: () => api<FirewallRule[]>(`/vps/servers/${id}/firewall`), enabled })
}

export function useAddFirewallRule(id: string) {
  return useServerMutation(id, 'firewall', (rule: Omit<FirewallRule, 'id'>) => api<void>(`/vps/servers/${id}/firewall`, { method: 'POST', body: rule }))
}

export function useDeleteFirewallRule(id: string) {
  return useServerMutation(id, 'firewall', (rule: string) => api<void>(`/vps/servers/${id}/firewall/${rule}`, { method: 'DELETE' }))
}

export function useSshKeys() {
  return useQuery({ queryKey: ['ssh-keys'], queryFn: () => api<SshKey[]>('/ssh-keys') })
}

export function useAddSshKey() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { name: string; public_key: string }) => api<SshKey>('/ssh-keys', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['ssh-keys'] }),
  })
}

export function useDeleteSshKey() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/ssh-keys/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['ssh-keys'] }),
  })
}

export function useShopProducts(kind: string) {
  return useQuery({ queryKey: ['shop', kind], queryFn: () => api<ShopProduct[]>(`/shop/products?kind=${kind}`) })
}

export type NewOrder = { product_id: string; billing_interval: string; name: string; image: number | null; ssh_key_ids: string[] }

export function usePlaceOrder() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: NewOrder) => api<{ invoice_id: number; subscription_code: string; service_id: string }>('/orders', { method: 'POST', body: input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['vps'] })
      queryClient.invalidateQueries({ queryKey: ['webhosting'] })
      queryClient.invalidateQueries({ queryKey: ['billing'] })
    },
  })
}

export function specsLine(specs: { cores?: number; memory_mb?: number; disk_gb?: number }) {
  const memory = specs.memory_mb ? (specs.memory_mb >= 1024 ? `${specs.memory_mb / 1024} GB` : `${specs.memory_mb} MB`) : null
  return [specs.cores ? `${specs.cores} vCPU` : null, memory ? `${memory} RAM` : null, specs.disk_gb ? `${specs.disk_gb} GB SSD` : null].filter(Boolean).join(' · ')
}

export type ServerSnapshot = Snapshot & { server_id: string; server_name: string }

export function useAllSnapshots() {
  return useQuery({ queryKey: ['vps', 'snapshots'], queryFn: () => api<{ snapshots: ServerSnapshot[]; used: number; limit: number }>('/vps/snapshots') })
}

export function useSnapshotAction() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ serverId, action, name, description }: { serverId: string; action: 'create' | 'rollback' | 'delete'; name?: string; description?: string }) =>
      action === 'create'
        ? api<void>(`/vps/servers/${serverId}/snapshots`, { method: 'POST', body: { description } })
        : action === 'rollback'
          ? api<void>(`/vps/servers/${serverId}/snapshots/${name}/rollback`, { method: 'POST' })
          : api<void>(`/vps/servers/${serverId}/snapshots/${name}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'snapshots'] }),
  })
}
