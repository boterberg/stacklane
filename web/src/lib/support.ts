import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import type { TicketStatus } from './labels'
import type { Paginated } from './pagination'

export type Department = { id: string; name: string }

export type TicketScope = 'customer' | 'staff'

const basePath: Record<TicketScope, string> = { customer: '/support/tickets', staff: '/admin/tickets' }

export type TicketSummary = {
  id: string
  number: number
  customer_name: string
  customer_email: string
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

export function useTickets(scope: TicketScope, { q, statuses, page, perPage }: TicketFilters) {
  const params = new URLSearchParams({ status: statuses.join(','), page: String(page), per_page: String(perPage) })
  if (q.trim()) params.set('q', q.trim())
  return useQuery({
    queryKey: ['support', 'tickets', scope, params.toString()],
    queryFn: () => api<Paginated<TicketSummary>>(`${basePath[scope]}?${params}`),
    placeholderData: keepPreviousData,
  })
}

export function useTicket(scope: TicketScope, number: string) {
  return useQuery({ queryKey: ['support', 'ticket', scope, number], queryFn: () => api<TicketDetail>(`${basePath[scope]}/${number}`) })
}

function useTicketMutation<TInput>(scope: TicketScope, request: (input: TInput) => Promise<TicketDetail>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: request,
    onSuccess: (ticket) => {
      queryClient.setQueryData(['support', 'ticket', scope, String(ticket.number)], ticket)
      queryClient.invalidateQueries({ queryKey: ['support', 'tickets'] })
      queryClient.invalidateQueries({ queryKey: ['dashboard'] })
    },
  })
}

export function useCreateTicket() {
  return useTicketMutation('customer', (input: { department_id: string | null; subject: string; body_html: string }) =>
    api<TicketDetail>('/support/tickets', { method: 'POST', body: input }),
  )
}

export function useReplyToTicket(scope: TicketScope, number: number) {
  return useTicketMutation(scope, (body_html: string) => api<TicketDetail>(`${basePath[scope]}/${number}/replies`, { method: 'POST', body: { body_html } }))
}

export function useCloseTicket(number: number) {
  return useTicketMutation('customer', () => api<TicketDetail>(`/support/tickets/${number}/close`, { method: 'POST' }))
}

export function useUpdateTicketStatus(number: number) {
  return useTicketMutation('staff', (status: TicketStatus) => api<TicketDetail>(`/admin/tickets/${number}/status`, { method: 'PATCH', body: { status } }))
}

export function ticketPath(scope: TicketScope, number: number) {
  return `${basePath[scope]}/${number}`
}
