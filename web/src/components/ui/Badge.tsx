import type { ReactNode } from 'react'

import { cn } from '../../lib/cn'

export type BadgeColor = 'zinc' | 'green' | 'amber' | 'red' | 'blue' | 'sky'

const colors: Record<BadgeColor, string> = {
  zinc: 'bg-zinc-400/15 text-zinc-700 dark:bg-zinc-400/40 dark:text-zinc-200',
  green: 'bg-green-400/20 text-green-800 dark:bg-green-400/40 dark:text-green-200',
  amber: 'bg-amber-400/25 text-amber-800 dark:bg-amber-400/40 dark:text-amber-200',
  red: 'bg-red-400/20 text-red-700 dark:bg-red-400/40 dark:text-red-200',
  blue: 'bg-blue-400/20 text-blue-800 dark:bg-blue-400/40 dark:text-blue-200',
  sky: 'bg-sky-400/20 text-sky-800 dark:bg-sky-400/40 dark:text-sky-200',
}

export function Badge({ color = 'zinc', children, className }: { color?: BadgeColor; children: ReactNode; className?: string }) {
  return <span className={cn('inline-flex items-center rounded-md px-2 py-1 text-sm leading-4 font-medium whitespace-nowrap', colors[color], className)}>{children}</span>
}
