import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'
import type { ServiceStatus } from './gameservers'

export type Integration = {
  id: string
  kind: 'pterodactyl' | 'proxmox' | 'openstack' | 'virtfusion'
  name: string
  base_url: string
  last_checked_at: string | null
  last_error: string | null
  created_at: string
}

export type Catalog = {
  locations: Array<{ id: number; name: string }>
  nests: Array<{ id: number; name: string; eggs: Array<{ id: number; name: string }> }>
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
  provisioning: { memory?: number; disk?: number; cpu?: number; egg_id?: number; location_id?: number }
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
    mutationFn: (input: { kind: string; name: string; base_url: string; credentials: Record<string, string> }) =>
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
