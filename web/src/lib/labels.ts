export type TicketStatus = 'open' | 'answered' | 'awaiting_reply' | 'in_progress' | 'closed'

export const ticketStatusLabels: Record<TicketStatus, string> = {
  open: 'Open',
  answered: 'Beantwoord',
  awaiting_reply: 'Wachten op antwoord',
  in_progress: 'In uitvoering',
  closed: 'Gesloten',
}
