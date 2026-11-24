import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { t } from './i18n'

export type ServiceStatus = 'pending' | 'provisioning' | 'active' | 'suspended' | 'terminated' | 'failed'

export type GameServer = {
  id: string
  name: string
  status: ServiceStatus
  details: {
    identifier?: string
    address?: string
    memory?: number
    disk?: number
    cpu?: number
    egg?: string
  }
  last_error: string | null
  product_name: string | null
  created_at: string
}

export type PowerState = 'running' | 'starting' | 'stopping' | 'offline'

export type GameServerDetail = GameServer & {
  install_state: string | null
  resources: {
    current_state: PowerState
    is_suspended: boolean
    resources: {
      memory_bytes: number
      cpu_absolute: number
      disk_bytes: number
      network_rx_bytes: number
      network_tx_bytes: number
      uptime: number
    }
  } | null
}

export type FileEntry = {
  name: string
  mode: string
  size: number
  is_file: boolean
  is_symlink: boolean
  mimetype: string
  modified_at: string
}

export type Backup = {
  uuid: string
  name: string
  bytes: number
  is_successful: boolean
  is_locked: boolean
  created_at: string
  completed_at: string | null
}

export function useGameServers() {
  return useQuery({ queryKey: ['gameservers'], queryFn: () => api<GameServer[]>('/gameservers'), refetchInterval: 10_000 })
}

export function useGameServer(id: string) {
  return useQuery({ queryKey: ['gameserver', id], queryFn: () => api<GameServerDetail>(`/gameservers/${id}`), refetchInterval: 5_000 })
}

export function usePower(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (signal: 'start' | 'stop' | 'restart' | 'kill') => api<void>(`/gameservers/${id}/power`, { method: 'POST', body: { signal } }),
    onSuccess: () => setTimeout(() => queryClient.invalidateQueries({ queryKey: ['gameserver', id] }), 800),
  })
}

export function useFiles(id: string, directory: string) {
  return useQuery({
    queryKey: ['gameserver', id, 'files', directory],
    queryFn: () => api<{ directory: string; entries: FileEntry[] }>(`/gameservers/${id}/files?directory=${encodeURIComponent(directory)}`),
  })
}

export function useFileContent(id: string, file: string | null) {
  return useQuery({
    queryKey: ['gameserver', id, 'file', file],
    queryFn: () => api<{ file: string; content: string }>(`/gameservers/${id}/files/content?file=${encodeURIComponent(file ?? '')}`),
    enabled: file !== null,
  })
}

export async function saveFile(id: string, file: string, content: string) {
  const response = await fetch(`/api/gameservers/${id}/files/content?file=${encodeURIComponent(file)}`, {
    method: 'PUT',
    credentials: 'include',
    headers: { 'content-type': 'text/plain' },
    body: content,
  })
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}))
    throw new Error(Object.values(payload.errors ?? {})[0] as string ?? payload.message ?? t('Opslaan mislukt.'))
  }
}

export function useBackups(id: string) {
  return useQuery({ queryKey: ['gameserver', id, 'backups'], queryFn: () => api<Backup[]>(`/gameservers/${id}/backups`), refetchInterval: 5_000 })
}

export function useCreateBackup(id: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (name: string) => api<Backup>(`/gameservers/${id}/backups`, { method: 'POST', body: { name } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['gameserver', id, 'backups'] }),
  })
}

export async function backupDownloadUrl(id: string, backup: string) {
  return (await api<{ url: string }>(`/gameservers/${id}/backups/${backup}/download`)).url
}

export async function consoleToken(id: string) {
  return api<{ token: string; socket: string }>(`/gameservers/${id}/websocket`)
}
