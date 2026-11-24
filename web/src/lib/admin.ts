import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import type { InvoiceDetail } from './billing'
import type { Paginated } from './pagination'
import type { ServiceStatus } from './gameservers'

export type Integration = {
  id: string
  kind: 'pterodactyl' | 'proxmox' | 'openstack' | 'virtfusion' | 'directadmin' | 'plesk' | 'openprovider' | 'cpanel'
  name: string
  base_url: string
  last_checked_at: string | null
  last_error: string | null
  created_at: string
}

export type Flavor = { id: string; name: string; cores: number; memory_mb: number; disk_gb: number }

export type Catalog = {
  locations?: Array<{ id: number; name: string }>
  nests?: Array<{ id: number; name: string; eggs: Array<{ id: number; name: string }> }>
  flavors?: Flavor[]
  packages?: Array<{ id: number; name: string; cores: number; memory_mb: number; disk_gb: number }>
  plans?: Array<{ id: string; name: string }>
}

export type Product = {
  id: string
  kind: string
  slug: string
  name: string
  description: string | null
  monthly_price_cents: number
  is_active: boolean
  integration_id: string | null
  integration_name: string | null
  provisioning: { memory?: number; disk?: number; cpu?: number; egg_id?: number; location_id?: number; plan?: string }
  service_count: number
}

export type Service = {
  id: string
  kind: string
  name: string
  status: ServiceStatus
  external_id: string | null
  details: { address?: string; egg?: string }
  last_error: string | null
  product_name: string | null
  integration_name: string | null
  customer_name: string
  customer_email: string
  created_at: string
}

export function useIntegrations() {
  return useQuery({ queryKey: ['admin', 'integrations'], queryFn: () => api<Integration[]>('/admin/integrations') })
}

export function useCreateIntegration() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { kind: string; name: string; base_url: string; credentials: Record<string, string>; settings?: Record<string, unknown> }) =>
      api<{ id: string; message: string }>('/admin/integrations', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'integrations'] }),
  })
}

export function useTestIntegration() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<{ ok: boolean; message: string }>(`/admin/integrations/${id}/test`, { method: 'POST' }),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ['admin', 'integrations'] }),
  })
}

export function useDeleteIntegration() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/admin/integrations/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'integrations'] }),
  })
}

export function useCatalog(integrationId: string | null) {
  return useQuery({
    queryKey: ['admin', 'catalog', integrationId],
    queryFn: () => api<Catalog>(`/admin/integrations/${integrationId}/catalog`),
    enabled: !!integrationId,
    staleTime: 5 * 60_000,
  })
}

export function useProducts() {
  return useQuery({ queryKey: ['admin', 'products'], queryFn: () => api<Product[]>('/admin/products') })
}

export function useCreateProduct() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: unknown) => api<{ id: string }>('/admin/products', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'products'] }),
  })
}

export function useServices() {
  return useQuery({ queryKey: ['admin', 'services'], queryFn: () => api<Service[]>('/admin/services'), refetchInterval: 5_000 })
}

export function useCreateService() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { customer_email: string; product_id: string; name: string }) => api<{ id: string }>('/admin/services', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'services'] }),
  })
}

export function useServiceAction() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, action }: { id: string; action: 'suspend' | 'unsuspend' | 'terminate' | 'retry' }) =>
      api<void>(`/admin/services/${id}/${action}`, { method: 'POST' }),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ['admin', 'services'] }),
  })
}

export function useCustomerSearch(query: string) {
  return useQuery({
    queryKey: ['admin', 'customers', query],
    queryFn: () => api<Array<{ id: string; email: string; name: string }>>(`/admin/customers?q=${encodeURIComponent(query)}`),
  })
}

export type AdminInvoice = {
  id: number
  status: 'open' | 'paid' | 'cancelled' | 'refunded'
  issued_at: string
  due_at: string
  total_cents: number
  customer_name: string
  customer_email: string
}

export type AdminInvoiceDetail = InvoiceDetail & {
  customer_email: string
  payments: Array<{ provider: string; status: string; amount_cents: number; method: string | null; created_at: string }>
}

export function useAdminInvoices({ q, statuses, page, perPage }: { q: string; statuses: string[]; page: number; perPage: number }) {
  const params = new URLSearchParams({ status: statuses.join(','), page: String(page), per_page: String(perPage) })
  if (q.trim()) params.set('q', q.trim())
  return useQuery({
    queryKey: ['admin', 'invoices', params.toString()],
    queryFn: () => api<Paginated<AdminInvoice>>(`/admin/invoices?${params}`),
    placeholderData: keepPreviousData,
  })
}

export function useAdminInvoice(id: string) {
  return useQuery({ queryKey: ['admin', 'invoice', id], queryFn: () => api<AdminInvoiceDetail>(`/admin/invoices/${id}`) })
}

export function useInvoiceAction(id: number) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (action: 'mark-paid' | 'cancel') => api<AdminInvoiceDetail>(`/admin/invoices/${id}/${action}`, { method: 'POST' }),
    onSuccess: (invoice) => {
      queryClient.setQueryData(['admin', 'invoice', String(id)], invoice)
      queryClient.invalidateQueries({ queryKey: ['admin', 'invoices'] })
    },
  })
}

export type Company = {
  name: string
  address: string | null
  email: string | null
  website: string | null
  coc_number: string | null
  vat_number: string | null
  iban: string | null
  bic: string | null
}

export type BillingSettings = {
  tax_rate_basis_points: number
  home_country: string
  oss: boolean
  payment_term_days: number
  renewal_notice_days: number
  suspend_after_days: number
}

export type PaymentSettings = {
  mollie_enabled: boolean
  mollie_configured: boolean
  stripe_enabled: boolean
  stripe_configured: boolean
  stripe_webhook_configured: boolean
  bank_transfer_enabled: boolean
  webhook_urls: { mollie: string; stripe: string }
}

export type PaymentSettingsUpdate = {
  mollie_enabled: boolean
  mollie_key?: string
  stripe_enabled: boolean
  stripe_secret_key?: string
  stripe_webhook_secret?: string
  bank_transfer_enabled: boolean
}

export function useSettings() {
  return useQuery({ queryKey: ['admin', 'settings'], queryFn: () => api<{ company: Company; billing: BillingSettings }>('/admin/settings') })
}

export function useSaveSettings() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: { company?: Company; billing?: BillingSettings }) => api<{ company: Company; billing: BillingSettings }>('/admin/settings', { method: 'PUT', body: input }),
    onSuccess: (settings) => {
      queryClient.setQueryData(['admin', 'settings'], settings)
      queryClient.invalidateQueries({ queryKey: ['meta'] })
    },
  })
}

export function usePaymentSettings() {
  return useQuery({ queryKey: ['admin', 'payment-settings'], queryFn: () => api<PaymentSettings>('/admin/payment-settings') })
}

export function useSavePaymentSettings() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: PaymentSettingsUpdate) => api<PaymentSettings>('/admin/payment-settings', { method: 'PUT', body: input }),
    onSuccess: (settings) => queryClient.setQueryData(['admin', 'payment-settings'], settings),
  })
}

export type DomainExtension = {
  id: string
  integration_id: string
  integration_name: string
  extension: string
  register_cents: number
  renew_cents: number
  transfer_cents: number
  is_active: boolean
  domains: number
}

export type DomainExtensionInput = Omit<DomainExtension, 'id' | 'integration_name' | 'domains'>

export function useDomainExtensions() {
  return useQuery({ queryKey: ['admin', 'domain-extensions'], queryFn: () => api<DomainExtension[]>('/admin/domain-extensions') })
}

export function useSaveDomainExtension() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...input }: DomainExtensionInput & { id?: string }) =>
      api<void>(id ? `/admin/domain-extensions/${id}` : '/admin/domain-extensions', { method: id ? 'PUT' : 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'domain-extensions'] }),
  })
}

export function useDeleteDomainExtension() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/admin/domain-extensions/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['admin', 'domain-extensions'] }),
  })
}
