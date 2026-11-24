export type PageMeta = {
  total: number
  page: number
  per_page: number
  from: number
  to: number
}

export type Paginated<T> = {
  data: T[]
  meta: PageMeta
}
