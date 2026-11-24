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
