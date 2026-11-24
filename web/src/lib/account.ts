import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type ApiToken = {
  id: string
  name: string
  prefix: string
  scopes: string[]
  created_at: string
  last_used_at: string | null
  expires_at: string | null
}

export type NewApiToken = { name: string; scopes: string[]; expires_in_days: number | null }

export function useApiTokens() {
  return useQuery({ queryKey: ['account', 'api-tokens'], queryFn: () => api<{ tokens: ApiToken[]; areas: string[] }>('/account/api-tokens') })
}

export function useCreateApiToken() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: NewApiToken) => api<ApiToken & { secret: string }>('/account/api-tokens', { method: 'POST', body: input }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['account', 'api-tokens'] }),
  })
}

export function useDeleteApiToken() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/account/api-tokens/${id}`, { method: 'DELETE' }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['account', 'api-tokens'] }),
  })
}

export function useChangePassword() {
  return useMutation({
    mutationFn: (input: { current_password: string; password: string }) => api<void>('/account/password', { method: 'PUT', body: input }),
  })
}

export function useLogoutOtherSessions() {
  return useMutation({
    mutationFn: (password: string) => api<void>('/account/sessions/logout-others', { method: 'POST', body: { password } }),
  })
}

export type TwoFactorStatus = { enabled: boolean; enabled_at: string | null; recovery_codes_left: number }

export function useTwoFactor() {
  return useQuery({ queryKey: ['account', 'two-factor'], queryFn: () => api<TwoFactorStatus>('/account/two-factor') })
}

export function useTwoFactorSetup() {
  return useMutation({ mutationFn: () => api<{ secret: string; uri: string; qr_svg: string }>('/account/two-factor/setup', { method: 'POST' }) })
}

function useTwoFactorAction<TInput, TResult>(path: string, body: (input: TInput) => unknown) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: TInput) => api<TResult>(`/account/two-factor/${path}`, { method: 'POST', body: body(input) }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['account', 'two-factor'] }),
  })
}

export function useEnableTwoFactor() {
  return useTwoFactorAction<string, { recovery_codes: string[] }>('enable', (code) => ({ code }))
}

export function useDisableTwoFactor() {
  return useTwoFactorAction<string, void>('disable', (password) => ({ password }))
}

export function useNewRecoveryCodes() {
  return useTwoFactorAction<string, { recovery_codes: string[] }>('recovery-codes', (password) => ({ password }))
}

export type AccountDetails = {
  first_name: string
  last_name: string
  email: string
  invoice_email: string | null
  phone: string | null
  is_business: boolean
  company_name: string | null
  coc_number: string | null
  tax_id: string | null
  country: string
  address_line1: string | null
  address_line2: string | null
  postal_code: string | null
  city: string | null
}

export function useAccount() {
  return useQuery({ queryKey: ['account', 'details'], queryFn: () => api<AccountDetails>('/account') })
}

export function useSaveAccount() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: AccountDetails) => api<AccountDetails>('/account', { method: 'PUT', body: input }),
    onSuccess: (account) => {
      queryClient.setQueryData(['account', 'details'], account)
      queryClient.invalidateQueries({ queryKey: ['me'] })
    },
  })
}

export type PaymentDefaults = { mode: 'manual' | 'automatic'; mandate_id: string | null }

export function usePaymentDefaults() {
  return useQuery({ queryKey: ['account', 'payment-defaults'], queryFn: () => api<PaymentDefaults>('/account/payment-defaults') })
}

export function useSavePaymentDefaults() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: PaymentDefaults) => api<PaymentDefaults>('/account/payment-defaults', { method: 'PUT', body: input }),
    onSuccess: (defaults) => {
      queryClient.setQueryData(['account', 'payment-defaults'], defaults)
      queryClient.invalidateQueries({ queryKey: ['billing'] })
    },
  })
}

export function useRevokeMandate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/billing/mandates/${id}`, { method: 'DELETE' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['billing'] })
      queryClient.invalidateQueries({ queryKey: ['account', 'payment-defaults'] })
    },
  })
}

export function useDeleteAccount() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (password: string) => api<void>('/account/delete', { method: 'POST', body: { password } }),
    onSuccess: () => {
      queryClient.clear()
      queryClient.setQueryData(['me'], null)
    },
  })
}
