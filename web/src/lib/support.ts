import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import type { TicketStatus } from './labels'
import type { Paginated } from './pagination'

export type Department = { id: string; name: string }

export type TicketSummary = {
  id: string
  number: number
  subject: string
  status: TicketStatus
  department: string
  message_count: number
  created_at: string
  last_reply_at: string
}

export type TicketMessage = {
  id: string
  is_staff: boolean
  author_name: string
  body_html: string
  created_at: string
}

export type TicketDetail = TicketSummary & { messages: TicketMessage[] }

export type TicketFilters = {
  q: string
  statuses: TicketStatus[]
  page: number
  perPage: number
}

export function useDepartments() {
  return useQuery({ queryKey: ['support', 'departments'], queryFn: () => api<Department[]>('/support/departments'), staleTime: 5 * 60_000 })
}

export function useTickets({ q, statuses, page, perPage }: TicketFilters) {
  const params = new URLSearchParams({ status: statuses.join(','), page: String(page), per_page: String(perPage) })
  if (q.trim()) params.set('q', q.trim())
  return useQuery({
    queryKey: ['support', 'tickets', params.toString()],
    queryFn: () => api<Paginated<TicketSummary>>(`/support/tickets?${params}`),
    placeholderData: keepPreviousData,
  })
}

export function useTicket(number: string) {
  return useQuery({ queryKey: ['support', 'ticket', number], queryFn: () => api<TicketDetail>(`/support/tickets/${number}`) })
}

function useTicketMutation<TInput>(request: (input: TInput) => Promise<TicketDetail>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: request,
    onSuccess: (ticket) => {
      queryClient.setQueryData(['support', 'ticket', String(ticket.number)], ticket)
      queryClient.invalidateQueries({ queryKey: ['support', 'tickets'] })
      queryClient.invalidateQueries({ queryKey: ['dashboard'] })
    },
  })
}

export function useCreateTicket() {
  return useTicketMutation((input: { department_id: string | null; subject: string; body_html: string }) =>
    api<TicketDetail>('/support/tickets', { method: 'POST', body: input }),
  )
}

export function useReplyToTicket(number: number) {
  return useTicketMutation((body_html: string) => api<TicketDetail>(`/support/tickets/${number}/replies`, { method: 'POST', body: { body_html } }))
}

export function useCloseTicket(number: number) {
  return useTicketMutation(() => api<TicketDetail>(`/support/tickets/${number}/close`, { method: 'POST' }))
}
