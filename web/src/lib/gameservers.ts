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
  owned: boolean
  owner_name: string | null
  created_at: string
}

export type GamePermission = 'console' | 'power' | 'files' | 'backups' | 'activity'

export const gamePermissions: GamePermission[] = ['console', 'power', 'files', 'backups', 'activity']

export type PowerState = 'running' | 'starting' | 'stopping' | 'offline'

export type GameServerDetail = GameServer & {
  access: { owner: boolean; permissions: GamePermission[] }
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

export type ActivityEntry = {
  id: string
  action: string
  detail: string | null
  actor: string | null
  ip: string | null
  created_at: string
}

export function useActivity(id: string) {
  return useQuery({ queryKey: ['gameserver', id, 'activity'], queryFn: () => api<ActivityEntry[]>(`/gameservers/${id}/activity`) })
}

export type GameMember = {
  id: string
  email: string
  name: string
  accepted: boolean
  permissions: GamePermission[]
  created_at: string
}

export type SharedGameServer = {
  service_id: string
  server: string
  owner: string
  accepted: boolean
  permissions: GamePermission[]
  created_at: string
}

export function useGameMembers(id: string) {
  return useQuery({ queryKey: ['gameserver', id, 'members'], queryFn: () => api<GameMember[]>(`/gameservers/${id}/members`) })
}

export function useGameMemberChanges(id: string) {
  const queryClient = useQueryClient()
  const onSuccess = () => queryClient.invalidateQueries({ queryKey: ['gameserver', id, 'members'] })
  return {
    invite: useMutation({ mutationFn: (input: { email: string; permissions: GamePermission[] }) => api<void>(`/gameservers/${id}/members`, { method: 'POST', body: input }), onSuccess }),
    update: useMutation({
      mutationFn: ({ member, permissions }: { member: string; permissions: GamePermission[] }) => api<void>(`/gameservers/${id}/members/${member}`, { method: 'PUT', body: { permissions } }),
      onSuccess,
    }),
    remove: useMutation({ mutationFn: (member: string) => api<void>(`/gameservers/${id}/members/${member}`, { method: 'DELETE' }), onSuccess }),
  }
}

export function useSharedGameServers() {
  return useQuery({ queryKey: ['shared-gameservers'], queryFn: () => api<SharedGameServer[]>('/account/shared-gameservers') })
}

export function useSharedGameServerChanges() {
  const queryClient = useQueryClient()
  const onSuccess = () => {
    queryClient.invalidateQueries({ queryKey: ['shared-gameservers'] })
    queryClient.invalidateQueries({ queryKey: ['gameservers'] })
  }
  return {
    accept: useMutation({ mutationFn: (server: string) => api<void>(`/account/shared-gameservers/${server}/accept`, { method: 'POST' }), onSuccess }),
    leave: useMutation({ mutationFn: (server: string) => api<void>(`/account/shared-gameservers/${server}`, { method: 'DELETE' }), onSuccess }),
  }
}

export function gamePermissionLabel(permission: GamePermission) {
  switch (permission) {
    case 'console':
      return t('Console')
    case 'power':
      return t('Starten en stoppen')
    case 'files':
      return t('Bestanden')
    case 'backups':
      return t('Back-ups')
    case 'activity':
      return t('Activiteit')
  }
}

export function gamePermissionHint(permission: GamePermission) {
  switch (permission) {
    case 'console':
      return t('Live meekijken en commando\'s sturen. Daarmee kan iemand de server ook stoppen.')
    case 'power':
      return t('De server starten, herstarten en stoppen.')
    case 'files':
      return t('Bestanden bekijken en wijzigen, ook configuratie en wachtwoorden in bestanden.')
    case 'backups':
      return t('Back-ups maken en downloaden. Een back-up bevat alle bestanden.')
    case 'activity':
      return t('Zien wie wat deed op de server, zonder IP-adressen.')
  }
}

export type AddonContext = { loader: string | null; version: string | null; loaders: string[] }

export type AddonHit = {
  project_id: string
  slug: string
  title: string
  description: string
  downloads: number
  icon_url: string | null
  author: string
}

export type InstalledAddons = { folder: string; files: { name: string; size: number; modified_at: string }[] }

export function useAddonContext(id: string) {
  return useQuery({ queryKey: ['gameserver', id, 'addons', 'context'], queryFn: () => api<AddonContext>(`/gameservers/${id}/addons/context`), staleTime: 60_000 })
}

export function useInstalledAddons(id: string, loader: string) {
  return useQuery({
    queryKey: ['gameserver', id, 'addons', 'installed', loader],
    queryFn: () => api<InstalledAddons>(`/gameservers/${id}/addons?${new URLSearchParams({ loader })}`),
    enabled: !!loader,
  })
}

export function useAddonSearch(id: string, loader: string, query: string, version: string) {
  return useQuery({
    queryKey: ['gameserver', id, 'addons', 'search', loader, query, version],
    queryFn: () => api<AddonHit[]>(`/gameservers/${id}/addons/search?${new URLSearchParams({ loader, query, ...(version ? { version } : {}) })}`),
    enabled: !!loader,
    staleTime: 60_000,
  })
}

export function useAddonChanges(id: string) {
  const queryClient = useQueryClient()
  const onSuccess = () => queryClient.invalidateQueries({ queryKey: ['gameserver', id, 'addons', 'installed'] })
  return {
    install: useMutation({
      mutationFn: (input: { project: string; loader: string; version: string }) =>
        api<{ title: string; file: string; folder: string }>(`/gameservers/${id}/addons`, { method: 'POST', body: { ...input, version: input.version || null } }),
      onSuccess: () => setTimeout(onSuccess, 1500),
    }),
    remove: useMutation({
      mutationFn: (input: { loader: string; file: string }) => api<void>(`/gameservers/${id}/addons?${new URLSearchParams(input)}`, { method: 'DELETE' }),
      onSuccess,
    }),
  }
}

const loaderNames: Record<string, string> = {
  paper: 'Paper',
  purpur: 'Purpur',
  folia: 'Folia',
  spigot: 'Spigot',
  bukkit: 'Bukkit',
  fabric: 'Fabric',
  quilt: 'Quilt',
  forge: 'Forge',
  neoforge: 'NeoForge',
}

export function loaderName(loader: string) {
  return loaderNames[loader] ?? loader
}

export function usesPlugins(loader: string) {
  return ['paper', 'purpur', 'folia', 'spigot', 'bukkit'].includes(loader)
}

export function isMinecraft(egg: string | undefined) {
  return !!egg && /minecraft|paper|purpur|folia|spigot|bukkit|fabric|quilt|forge/i.test(egg)
}
