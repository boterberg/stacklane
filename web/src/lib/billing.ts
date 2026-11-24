import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import { formatRelative } from './format'
import { t } from './i18n'
import type { Paginated } from './pagination'

export type SubscriptionStatus = 'pending' | 'active' | 'suspended' | 'cancelled' | 'expired'
export type InvoiceStatus = 'open' | 'paid' | 'cancelled' | 'refunded'

export type SubscriptionSummary = {
  id: string
  code: string
  name: string
  status: SubscriptionStatus
  billing_interval: 'monthly' | 'quarterly' | 'half_yearly' | 'yearly'
  payment_mode: 'manual' | 'automatic'
  monthly_total_cents: number
  expires_at: string
  cancelled_at: string | null
}

export type InvoiceSummary = {
  id: number
  status: InvoiceStatus
  issued_at: string
  due_at: string
  total_cents: number
}

export type SubscriptionDetail = SubscriptionSummary & {
  started_at: string
  mandate_id: string | null
  mandate: string | null
  items: Array<{ description: string; specs_line: string | null; monthly_price_cents: number }>
  invoices: InvoiceSummary[]
}

export type InvoiceDetail = InvoiceSummary & {
  tax_cents: number
  tax_rate_basis_points: number
  paid_at: string | null
  customer: {
    name?: string
    company_name?: string | null
    address_line1?: string | null
    address_line2?: string | null
    postal_code?: string | null
    city?: string | null
    country?: string | null
  }
  lines: Array<{ description: string; period_start: string | null; period_end: string | null; amount_cents: number; subscription_code: string | null }>
}

type Filters = { q: string; statuses?: string[]; page: number; perPage: number }

function query({ q, statuses, page, perPage }: Filters) {
  const params = new URLSearchParams({ page: String(page), per_page: String(perPage) })
  if (statuses) params.set('status', statuses.join(','))
  if (q.trim()) params.set('q', q.trim())
  return params.toString()
}

export function renews(subscription: SubscriptionSummary) {
  return subscription.payment_mode === 'automatic' && subscription.status === 'active' && !subscription.cancelled_at
}

export function useSubscriptions(filters: Filters) {
  const params = query(filters)
  return useQuery({
    queryKey: ['billing', 'subscriptions', params],
    queryFn: () => api<Paginated<SubscriptionSummary>>(`/billing/subscriptions?${params}`),
    placeholderData: keepPreviousData,
  })
}

export function useSubscription(code: string) {
  return useQuery({ queryKey: ['billing', 'subscription', code], queryFn: () => api<SubscriptionDetail>(`/billing/subscriptions/${code}`) })
}

export function useInvoices(filters: Filters) {
  const params = query(filters)
  return useQuery({
    queryKey: ['billing', 'invoices', params],
    queryFn: () => api<Paginated<InvoiceSummary>>(`/billing/invoices?${params}`),
    placeholderData: keepPreviousData,
  })
}

export function useInvoice(id: string, waitForPayment = false) {
  return useQuery({
    queryKey: ['billing', 'invoice', id],
    queryFn: () => api<InvoiceDetail>(`/billing/invoices/${id}`),
    refetchInterval: (query) => (waitForPayment && query.state.data?.status === 'open' ? 3000 : false),
  })
}

export type Mandate = { id: string; provider: 'mollie' | 'stripe'; description: string; created_at: string }

export type PaymentOptions = {
  providers: { mollie: boolean; stripe: boolean; bank_transfer: boolean }
  iban: string | null
  account_holder: string | null
  mandates: Mandate[]
}

export function usePaymentOptions() {
  return useQuery({ queryKey: ['billing', 'payment-options'], queryFn: () => api<PaymentOptions>('/billing/payment-options') })
}

export function usePayInvoice(id: number) {
  return useMutation({
    mutationFn: (input: { provider: 'mollie' | 'stripe'; save: boolean }) => api<{ checkout_url: string }>(`/billing/invoices/${id}/pay`, { method: 'POST', body: input }),
    onSuccess: ({ checkout_url }) => {
      window.location.href = checkout_url
    },
  })
}

function useSubscriptionMutation<TInput>(code: string, request: (input: TInput) => Promise<SubscriptionDetail>) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: request,
    onSuccess: (subscription) => {
      queryClient.setQueryData(['billing', 'subscription', code], subscription)
      queryClient.invalidateQueries({ queryKey: ['billing', 'subscriptions'] })
    },
  })
}

export function useCancelSubscription(code: string) {
  return useSubscriptionMutation(code, () => api<SubscriptionDetail>(`/billing/subscriptions/${code}/cancel`, { method: 'POST' }))
}

export function useResumeSubscription(code: string) {
  return useSubscriptionMutation(code, () => api<SubscriptionDetail>(`/billing/subscriptions/${code}/resume`, { method: 'POST' }))
}

export function useSetPaymentMethod(code: string) {
  return useSubscriptionMutation(code, (mandateId: string | null) =>
    api<SubscriptionDetail>(`/billing/subscriptions/${code}/payment-method`, { method: 'PUT', body: { mandate_id: mandateId } }),
  )
}

export function invoicePdfUrl(id: number) {
  return `/api/billing/invoices/${id}/pdf`
}

export function hasExpired(subscription: SubscriptionSummary) {
  return new Date(subscription.expires_at).getTime() < Date.now()
}

export function expiryText(subscription: SubscriptionSummary) {
  const when = formatRelative(subscription.expires_at)
  const text = hasExpired(subscription) ? t('{when} afgelopen', { when }) : t('Verloopt {when}', { when })
  return text.charAt(0).toUpperCase() + text.slice(1)
}

export function renewalText(subscription: SubscriptionSummary) {
  if (renews(subscription)) return t('Wordt automatisch verlengd')
  if (subscription.status === 'cancelled' || subscription.status === 'expired' || subscription.cancelled_at) return t('Wordt niet verlengd')
  return t('Verlengen na een herinnering')
}
