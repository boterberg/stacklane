import type { ReactNode } from 'react'

import { LogoSymbol } from './Logo'

type EmptyStateProps = {
  title: string
  description: string
  action?: ReactNode
}

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center rounded-xl border border-dashed border-zinc-300 px-6 py-12 text-center dark:border-white/20">
      <LogoSymbol className="mb-3 size-9 text-zinc-400 dark:text-zinc-500" />
      <h3 className="text-base font-medium text-zinc-800 dark:text-white">{title}</h3>
      <p className="text-sm text-zinc-500 dark:text-white/70">{description}</p>
      {action && <div className="mt-4">{action}</div>}
    </div>
  )
}
