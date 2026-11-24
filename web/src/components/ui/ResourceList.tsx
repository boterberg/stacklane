import { ChevronLeftIcon, ChevronRightIcon } from '@heroicons/react/20/solid'
import type { ComponentType, ReactNode, SVGProps } from 'react'
import { Link } from 'react-router'

import type { PageMeta } from '../../lib/pagination'
import { t } from '../../lib/i18n'

export type MetaItem = {
  icon: ComponentType<SVGProps<SVGSVGElement>>
  label: ReactNode
}

type RowCardProps = {
  to: string
  title: ReactNode
  badge?: ReactNode
  meta: MetaItem[]
  aside?: ReactNode
}

export function RowCard({ to, title, badge, meta, aside }: RowCardProps) {
  return (
    <li className="relative flex items-center gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-[22px] transition-colors hover:bg-zinc-50 dark:border-white/10 dark:bg-white/10 dark:hover:bg-white/[0.13]">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-3">
          <Link to={to} className="truncate text-sm font-medium text-zinc-800 after:absolute after:inset-0 dark:text-white">
            {title}
          </Link>
          {badge}
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-x-6 gap-y-1">
          {meta.map(({ icon: Icon, label }, index) => (
            <span key={index} className="flex items-center gap-2 text-sm text-zinc-500 dark:text-white/70">
              <Icon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {label}
            </span>
          ))}
        </div>
      </div>
      {aside && <div className="relative shrink-0">{aside}</div>}
    </li>
  )
}

export function RowCardList({ children }: { children: ReactNode }) {
  return <ul className="space-y-3">{children}</ul>
}

export type Column<T> = {
  header: string
  cell: (row: T) => ReactNode
  className?: string
}

export function DataTable<T>({ rows, columns, rowKey }: { rows: T[]; columns: Column<T>[]; rowKey: (row: T) => string }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-left text-sm">
        <thead>
          <tr className="border-b border-zinc-800/10 dark:border-white/20">
            {columns.map((column) => (
              <th key={column.header} className={`py-3 pr-4 font-medium text-zinc-800 dark:text-white ${column.className ?? ''}`}>
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={rowKey(row)} className="border-b border-zinc-800/10 dark:border-white/10">
              {columns.map((column) => (
                <td key={column.header} className={`py-3 pr-4 text-zinc-500 dark:text-zinc-300 ${column.className ?? ''}`}>
                  {column.cell(row)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

export function PaginationFooter({ meta, onPage }: { meta: PageMeta; onPage: (page: number) => void }) {
  const lastPage = Math.max(1, Math.ceil(meta.total / meta.per_page))
  const pageButton = 'flex size-8 items-center justify-center rounded-md text-zinc-500 hover:bg-zinc-800/5 disabled:opacity-40 disabled:hover:bg-transparent dark:text-white/70 dark:hover:bg-white/10'
  return (
    <div className="mt-6 flex items-center justify-between border-t border-zinc-800/10 pt-3 dark:border-white/10">
      <p className="text-xs font-medium text-zinc-500 dark:text-zinc-400">
        {t('Toont {from} tot {to} van {total} resultaten', { from: meta.from, to: meta.to, total: meta.total })}
      </p>
      {lastPage > 1 && (
        <div className="flex items-center gap-1">
          <button type="button" className={pageButton} disabled={meta.page <= 1} onClick={() => onPage(meta.page - 1)} aria-label={t('Vorige pagina')}>
            <ChevronLeftIcon className="size-4" />
          </button>
          <span className="px-2 text-xs font-medium text-zinc-500 dark:text-zinc-400">
            {meta.page} / {lastPage}
          </span>
          <button type="button" className={pageButton} disabled={meta.page >= lastPage} onClick={() => onPage(meta.page + 1)} aria-label={t('Volgende pagina')}>
            <ChevronRightIcon className="size-4" />
          </button>
        </div>
      )}
    </div>
  )
}
