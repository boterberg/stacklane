import type { BadgeColor } from '../components/ui/Badge'
import { t } from './i18n'

export function translated<T extends string>(labels: Record<T, string>): Record<T, string> {
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

export const subscriptionStatuses = ['active', 'pending', 'suspended', 'cancelled', 'expired'] as const

export const subscriptionStatusLabels: Record<string, string> = translated({
  pending: 'In afwachting',
  active: 'Actief',
  suspended: 'Opgeschort',
  cancelled: 'Geannuleerd',
  expired: 'Verlopen',
})

export const subscriptionStatusColors: Record<string, BadgeColor> = {
  pending: 'sky',
  active: 'green',
  suspended: 'amber',
  cancelled: 'zinc',
  expired: 'zinc',
}

export const invoiceStatusLabels: Record<string, string> = translated({
  open: 'Openstaand',
  paid: 'Betaald',
  cancelled: 'Geannuleerd',
  refunded: 'Terugbetaald',
})

export const invoiceStatusColors: Record<string, BadgeColor> = {
  open: 'amber',
  paid: 'green',
  cancelled: 'zinc',
  refunded: 'blue',
}

export const intervalLabels: Record<string, string> = translated({
  monthly: 'Maandelijks',
  quarterly: 'Per kwartaal',
  half_yearly: 'Per half jaar',
  yearly: 'Jaarlijks',
})
