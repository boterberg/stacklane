import type { ReactNode } from 'react'

import { cn } from '../../lib/cn'

type CardProps = {
  title: ReactNode
  description?: ReactNode
  action?: ReactNode
  children: ReactNode
  className?: string
}

export function Card({ title, description, action, children, className }: CardProps) {
  return (
    <section className={cn('rounded-xl border border-zinc-200 bg-zinc-50 dark:border-white/7 dark:bg-white/5', className)}>
      <header className={cn('flex min-h-14 items-center gap-4 pl-6', description ? 'py-4' : 'py-3', action ? 'pr-3' : 'pr-6')}>
        <div className="min-w-0 flex-1">
          <h2 className="text-sm leading-6 font-medium text-zinc-800 dark:text-white">{title}</h2>
          {description && <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{description}</p>}
        </div>
        {action}
      </header>
      <div className="-m-px rounded-xl border border-zinc-200 bg-white p-6 dark:border-white/7 dark:bg-white/4">{children}</div>
    </section>
  )
}

export function CardText({ children }: { children: ReactNode }) {
  return <p className="text-sm text-zinc-500 dark:text-white/70">{children}</p>
}
