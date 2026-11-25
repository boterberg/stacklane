import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api, currentVpsOwner, setVpsOwner } from './api'

export type VpsAccess = { owner_id: string; is_owner: boolean; owner_name: string; permissions: string[] }
export type Member = { id: string; email: string; name: string; accepted: boolean; permissions: string[]; created_at: string }
export type SharedEnvironment = { owner_id: string; email: string; name: string; accepted: boolean; permissions: string[]; created_at: string }

export const permissionGroups: Array<{ id: string; label: string; rights: Array<[string, string]> }> = [
  {
    id: 'servers',
    label: 'Servers',
    rights: [
      ['view', 'Bekijken'],
      ['start', 'Start'],
      ['stop', 'Stop'],
      ['rename', 'Hernoemen'],
      ['console', 'VNC-console gebruiken'],
      ['install', 'Installeer'],
      ['rebuild', 'Herbouwen'],
      ['rescue', 'Rescue-modus gebruiken'],
      ['restore', 'Herstellen vanaf snapshot'],
      ['rdns', 'Reverse DNS beheren'],
    ],
  },
  {
    id: 'snapshots',
    label: 'Snapshots',
    rights: [
      ['view', 'Bekijken'],
      ['create', 'Maken'],
      ['schedule', 'Automatisch inplannen'],
      ['rename', 'Hernoemen'],
      ['delete', 'Verwijderen'],
    ],
  },
  {
    id: 'volumes',
    label: 'Volumes',
    rights: [
      ['view', 'Bekijken'],
      ['attach', 'Koppelen'],
      ['detach', 'Loskoppelen'],
      ['rename', 'Hernoemen'],
      ['resize', 'Vergroten'],
    ],
  },
  {
    id: 'networks',
    label: 'Netwerken',
    rights: [
      ['view', 'Bekijken'],
      ['create', 'Maken'],
      ['update', 'Update'],
      ['delete', 'Verwijderen'],
      ['subnet_create', 'Subnets maken'],
      ['subnet_delete', 'Subnets verwijderen'],
      ['attach', 'Servers koppelen'],
      ['detach', 'Servers loskoppelen'],
    ],
  },
  {
    id: 'firewalls',
    label: 'Firewalls',
    rights: [
      ['view', 'Bekijken'],
      ['create', 'Maken'],
      ['update', 'Update'],
      ['delete', 'Verwijderen'],
      ['rule_create', 'Regels maken'],
      ['rule_update', 'Regels bijwerken'],
      ['rule_delete', 'Regels verwijderen'],
      ['attach', 'Aan een server koppelen'],
      ['detach', 'Van een server loskoppelen'],
    ],
  },
  {
    id: 'floating_ips',
    label: 'Floating IPs',
    rights: [
      ['view', 'Bekijken'],
      ['attach', 'Koppelen'],
      ['detach', 'Ontkoppelen'],
    ],
  },
  {
    id: 'isos',
    label: "ISO's",
    rights: [
      ['view', 'Bekijken'],
      ['import', 'Importeren'],
      ['delete', 'Verwijderen'],
      ['mount', 'Koppelen en loskoppelen'],
    ],
  },
  {
    id: 'ssh_keys',
    label: 'SSH-sleutels',
    rights: [
      ['view', 'Bekijken'],
      ['create', 'Toevoegen'],
      ['delete', 'Verwijderen'],
    ],
  },
  {
    id: 'access',
    label: 'Toegang en support',
    rights: [
      ['users', 'Gebruikers beheren'],
      ['tickets', 'Support tickets maken'],
    ],
  },
]

export function useVpsAccess() {
  return useQuery({ queryKey: ['vps', 'access', currentVpsOwner()], queryFn: () => api<VpsAccess>('/vps/access'), staleTime: 60_000 })
}

export function useCan() {
  const { data } = useVpsAccess()
  return (permission: string) => !data || data.is_owner || data.permissions.includes(permission)
}

export function useSwitchEnvironment() {
  const queryClient = useQueryClient()
  return (owner: string | null) => {
    setVpsOwner(owner)
    queryClient.removeQueries({ queryKey: ['vps'] })
    queryClient.removeQueries({ queryKey: ['ssh-keys'] })
  }
}

export function useMembers(enabled: boolean) {
  return useQuery({ queryKey: ['vps', 'members'], queryFn: () => api<Member[]>('/vps/members'), enabled })
}

export function useSaveMember() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, email, permissions }: { id?: string; email: string; permissions: string[] }) =>
      id ? api<void>(`/vps/members/${id}`, { method: 'PUT', body: { permissions } }) : api<void>('/vps/members', { method: 'POST', body: { email, permissions } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'members'] }),
  })
}

export function useRemoveMember() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/vps/members/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'members'] }),
  })
}

export function useSharedEnvironments() {
  return useQuery({ queryKey: ['shared-access'], queryFn: () => api<SharedEnvironment[]>('/account/shared-access') })
}

export function useAcceptInvitation() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (owner: string) => api<void>(`/account/shared-access/${owner}/accept`, { method: 'POST' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['shared-access'] }),
  })
}

export function useLeaveEnvironment() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (owner: string) => api<void>(`/account/shared-access/${owner}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['shared-access'] }),
  })
}
