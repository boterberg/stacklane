import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { api } from './api'

export type ArticleSummary = {
  id: string
  slug: string
  title: string
  category: string
  excerpt: string
  published: boolean
  updated_at: string
}

export type Article = {
  id: string
  slug: string
  title: string
  category: string
  body_html: string
  published: boolean
  updated_at: string
}

export type ArticleInput = { title: string; category: string; body_html: string; published: boolean }

function query(search: string) {
  return search.trim() ? `?search=${encodeURIComponent(search.trim())}` : ''
}

export function useArticles(search: string) {
  return useQuery({ queryKey: ['kb', search], queryFn: () => api<ArticleSummary[]>(`/kb${query(search)}`), placeholderData: keepPreviousData })
}

export function useArticle(slug: string) {
  return useQuery({ queryKey: ['kb', 'article', slug], queryFn: () => api<Article>(`/kb/${slug}`) })
}

export function useAdminArticles(search: string) {
  return useQuery({ queryKey: ['admin', 'kb', search], queryFn: () => api<ArticleSummary[]>(`/admin/kb${query(search)}`), placeholderData: keepPreviousData })
}

export function useAdminArticle(id: string | null) {
  return useQuery({ queryKey: ['admin', 'kb', 'article', id], queryFn: () => api<Article>(`/admin/kb/${id}`), enabled: !!id })
}

export function useSaveArticle(id: string | null) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: ArticleInput) => (id ? api<Article>(`/admin/kb/${id}`, { method: 'PUT', body: input }) : api<Article>('/admin/kb', { method: 'POST', body: input })),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'kb'] })
      queryClient.invalidateQueries({ queryKey: ['kb'] })
    },
  })
}

export function useDeleteArticle() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api<void>(`/admin/kb/${id}`, { method: 'DELETE' }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin', 'kb'] })
      queryClient.invalidateQueries({ queryKey: ['kb'] })
    },
  })
}

export function groupByCategory<T extends { category: string }>(articles: T[], fallback: string) {
  const groups = new Map<string, T[]>()
  for (const article of articles) {
    const name = article.category || fallback
    groups.set(name, [...(groups.get(name) ?? []), article])
  }
  return [...groups.entries()]
}
