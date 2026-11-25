import type { ReactNode } from 'react'

import { cn } from '../../lib/cn'

type PageHeaderProps = {
  title: ReactNode
  description?: ReactNode
  actions?: ReactNode
}

export function PageHeader({ title, description, actions }: PageHeaderProps) {
  return (
    <div className={cn('flex flex-wrap items-center justify-between gap-4', actions ? 'mb-6' : 'mb-8')}>
      <div>
        <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{title}</h1>
        {description && <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{description}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  )
}
