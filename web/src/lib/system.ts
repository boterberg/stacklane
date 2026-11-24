import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type UpdateProgress = {
  state: 'queued' | 'running' | 'done' | 'failed'
  step: 'queued' | 'checking' | 'backup' | 'installing' | 'restarting' | 'done' | 'failed'
  target: string
  message: string | null
  updated_at: string
}

export type UpdateOverview = {
  current: string
  latest: string | null
  available: boolean
  checked_at: string | null
  can_update: boolean
  progress: UpdateProgress | null
}

const updateKey = ['system', 'update'] as const

export function isRunning(progress: UpdateProgress | null | undefined) {
  if (!progress || (progress.state !== 'queued' && progress.state !== 'running')) return false
  return Date.now() - new Date(progress.updated_at).getTime() < 30 * 60_000
}

export function justFinished(overview: UpdateOverview) {
  const progress = overview.progress
  if (!progress || progress.state !== 'done' || progress.target !== overview.current) return false
  return Date.now() - new Date(progress.updated_at).getTime() < 60 * 60_000
}

export function needsAttention(overview: UpdateOverview | undefined) {
  return !!overview && (overview.available || isRunning(overview.progress) || justFinished(overview))
}

export function useUpdateStatus(enabled: boolean) {
  return useQuery({
    queryKey: updateKey,
    queryFn: () => api<UpdateOverview>('/admin/system/update'),
    enabled,
    staleTime: 10 * 60_000,
    retry: (count) => count < 2,
    refetchInterval: (query) => (isRunning(query.state.data?.progress) ? 2000 : false),
    refetchIntervalInBackground: true,
  })
}

export function useStartUpdate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => api<UpdateOverview>('/admin/system/update', { method: 'POST' }),
    onSuccess: (overview) => queryClient.setQueryData(updateKey, overview),
  })
}

export function useCheckForUpdate() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: () => api<UpdateOverview>('/admin/system/update/check', { method: 'POST' }),
    onSuccess: (overview) => queryClient.setQueryData(updateKey, overview),
  })
}

export function releaseNotesUrl(version: string) {
  return `https://stacklane.io/releases/v${version}`
}
