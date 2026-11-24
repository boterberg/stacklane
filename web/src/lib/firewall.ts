import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import type { FirewallRule } from './vps'

export type NewRule = { type: 'in' | 'out'; action: string; proto: string; dport: string; source: string; comment: string }

export const emptyRule: NewRule = { type: 'in', action: 'ACCEPT', proto: 'tcp', dport: '', source: '', comment: '' }

const marker = /^\[fw:[0-9a-f]{8}\]\s?/

export function ruleNote(comment?: string | null) {
  return comment?.replace(marker, '') || null
}

export function isManaged(rule: FirewallRule) {
  return marker.test(rule.comment ?? '')
}

export type AccountFirewall = { id: string; name: string; auto_assign: boolean; rules: number; servers: number; created_at: string }

export type FirewallDetail = AccountFirewall & { rule_list: FirewallRule[]; server_list: { id: string; name: string }[] }

export type FirewallInput = { name: string; auto_assign: boolean; rules: Omit<FirewallRule, 'id'>[]; server_ids?: string[] }

export type SaveResult = { id: string; problems: { server: string; error: string }[] }

export function useFirewalls() {
  return useQuery({ queryKey: ['vps', 'firewalls'], queryFn: () => api<AccountFirewall[]>('/vps/firewalls') })
}

export function useFirewallDetail(id: string) {
  return useQuery({ queryKey: ['vps', 'firewalls', id], queryFn: () => api<FirewallDetail>(`/vps/firewalls/${id}`) })
}

export function useFirewallServers() {
  return useQuery({ queryKey: ['vps', 'firewalls', 'servers'], queryFn: () => api<{ id: string; name: string }[]>('/vps/firewalls/servers') })
}

export function useSaveFirewall() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, input }: { id?: string; input: FirewallInput }) =>
      id ? api<SaveResult>(`/vps/firewalls/${id}`, { method: 'PUT', body: input }) : api<SaveResult>('/vps/firewalls', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps'] }),
  })
}

export function useDeleteFirewall() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<SaveResult>(`/vps/firewalls/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps'] }),
  })
}
