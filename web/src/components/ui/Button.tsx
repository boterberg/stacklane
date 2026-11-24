import type { ComponentProps, ComponentType, ReactNode, SVGProps } from 'react'
import { Link } from 'react-router'

import { cn } from '../../lib/cn'

type Variant = 'primary' | 'outline' | 'ghost' | 'danger'
type Size = 'base' | 'sm'

const variants: Record<Variant, string> = {
  primary:
    'bg-zinc-800 text-white hover:bg-zinc-900 shadow-[inset_0px_1px_theme(--color-white/0.2)] dark:bg-white dark:text-zinc-800 dark:hover:bg-zinc-100 dark:shadow-none',
  outline:
    'border border-zinc-200 border-b-zinc-300/80 bg-white text-zinc-800 shadow-xs hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75',
  ghost: 'text-zinc-800 hover:bg-zinc-800/5 dark:text-white dark:hover:bg-white/15',
  danger: 'bg-red-500 text-white shadow-[inset_0px_1px_theme(--color-red-500),inset_0px_2px_theme(--color-white/0.15)] hover:bg-red-600 dark:bg-red-600 dark:hover:bg-red-500',
}

const sizes: Record<Size, string> = {
  base: 'h-10 gap-2 rounded-lg px-4 text-sm',
  sm: 'h-8 gap-2 rounded-md px-3 text-sm',
}

type Common = {
  variant?: Variant
  size?: Size
  icon?: ComponentType<SVGProps<SVGSVGElement>>
  children?: ReactNode
  className?: string
}

function classes({ variant = 'outline', size = 'base', className }: Common) {
  return cn(
    'inline-flex shrink-0 items-center justify-center font-medium whitespace-nowrap transition-colors disabled:opacity-75 disabled:cursor-default',
    variants[variant],
    sizes[size],
    className,
  )
}

function Content({ icon: Icon, children }: Common) {
  return (
    <>
      {Icon && <Icon className="-ml-0.5 size-4 shrink-0" aria-hidden />}
      {children}
    </>
  )
}

export function Button({ variant, size, icon, className, children, type = 'button', ...rest }: Common & ComponentProps<'button'>) {
  return (
    <button type={type} className={classes({ variant, size, className })} {...rest}>
      <Content icon={icon}>{children}</Content>
    </button>
  )
}

export function ButtonLink({ variant, size, icon, className, children, to }: Common & { to: string }) {
  return (
    <Link to={to} className={classes({ variant, size, className })}>
      <Content icon={icon}>{children}</Content>
    </Link>
  )
}
