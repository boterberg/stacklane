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
