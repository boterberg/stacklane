import type { BadgeColor } from '../components/ui/Badge'
import { t } from './i18n'

function translated<T extends string>(labels: Record<T, string>): Record<T, string> {
  return new Proxy(labels, {
    get: (target, key) => (typeof key === 'string' && key in target ? t(target[key as T]) : undefined),
  })
}

export type TicketStatus = 'open' | 'answered' | 'awaiting_reply' | 'in_progress' | 'closed'

export const ticketStatuses: TicketStatus[] = ['answered', 'open', 'awaiting_reply', 'in_progress', 'closed']

export const defaultTicketStatuses: TicketStatus[] = ['answered', 'open', 'awaiting_reply', 'in_progress']

export const ticketStatusLabels: Record<TicketStatus, string> = translated({
  open: 'Open',
  answered: 'Beantwoord',
  awaiting_reply: 'Wachten op antwoord',
  in_progress: 'In uitvoering',
  closed: 'Gesloten',
})

export const ticketStatusColors: Record<TicketStatus, BadgeColor> = {
  open: 'blue',
  answered: 'green',
  awaiting_reply: 'amber',
  in_progress: 'sky',
  closed: 'zinc',
}

export const serviceStatusLabels: Record<string, string> = translated({
  pending: 'In de wachtrij',
  provisioning: 'Wordt aangemaakt',
  active: 'Actief',
  suspended: 'Opgeschort',
  terminated: 'Opgeheven',
  failed: 'Mislukt',
})

export const serviceStatusColors: Record<string, BadgeColor> = {
  pending: 'zinc',
  provisioning: 'sky',
  active: 'green',
  suspended: 'amber',
  terminated: 'zinc',
  failed: 'red',
}

export const powerStateLabels: Record<string, string> = translated({
  running: 'Online',
  starting: 'Start op',
  stopping: 'Stopt',
  offline: 'Offline',
})

export const powerStateColors: Record<string, BadgeColor> = {
  running: 'green',
  starting: 'sky',
  stopping: 'amber',
  offline: 'zinc',
}
