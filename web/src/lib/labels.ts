import type { BadgeColor } from '../components/ui/Badge'

export type TicketStatus = 'open' | 'answered' | 'awaiting_reply' | 'in_progress' | 'closed'

export const ticketStatuses: TicketStatus[] = ['answered', 'open', 'awaiting_reply', 'in_progress', 'closed']

export const defaultTicketStatuses: TicketStatus[] = ['answered', 'open', 'awaiting_reply', 'in_progress']

export const ticketStatusLabels: Record<TicketStatus, string> = {
  open: 'Open',
  answered: 'Beantwoord',
  awaiting_reply: 'Wachten op antwoord',
  in_progress: 'In uitvoering',
  closed: 'Gesloten',
}

export const ticketStatusColors: Record<TicketStatus, BadgeColor> = {
  open: 'blue',
  answered: 'green',
  awaiting_reply: 'amber',
  in_progress: 'sky',
  closed: 'zinc',
}
