import { useQuery } from '@tanstack/react-query'

import { api } from './api'

export type NotificationList = {
  unread_count: number
  items: Array<{ id: string; title: string; body: string; link: string | null; created_at: string }>
}

export function useNotifications() {
  return useQuery({ queryKey: ['notifications'], queryFn: () => api<NotificationList>('/notifications'), refetchInterval: 60_000 })
}
