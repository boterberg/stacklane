import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type Iso = {
  id: string
  name: string
  url: string
  firmware: 'bios' | 'uefi'
  status: 'importing' | 'ready' | 'failed'
  last_error: string | null
  mounted_on: string | null
  server_name: string | null
  created_at: string
}

export function useIsos() {
  return useQuery({
    queryKey: ['vps', 'isos'],
    queryFn: () => api<{ isos: Iso[]; used: number; limit: number; can_import: boolean }>('/vps/isos'),
    refetchInterval: (query) => (query.state.data?.isos.some((iso) => iso.status === 'importing') ? 5000 : false),
  })
}

export function useImportIso() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { name: string; url: string; firmware: string }) => api<Iso>('/vps/isos', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'isos'] }),
  })
}

export function useDeleteIso() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/vps/isos/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'isos'] }),
  })
}

export function useServerIso(serverId: string) {
  return useQuery({ queryKey: ['vps', 'isos', 'server', serverId], queryFn: () => api<{ mounted: Iso | null; available: Iso[] }>(`/vps/servers/${serverId}/iso`) })
}

export function useMountIso(serverId: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (isoId: string | null) =>
      isoId ? api<void>(`/vps/servers/${serverId}/iso`, { method: 'POST', body: { iso_id: isoId } }) : api<void>(`/vps/servers/${serverId}/iso`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'isos'] }),
  })
}
