import { useId, type ComponentProps, type ReactNode } from 'react'

import { cn } from '../../lib/cn'

type FieldProps = {
  label: ReactNode
  description?: ReactNode
  error?: string
  aside?: ReactNode
  children: (id: string) => ReactNode
}

export function Field({ label, description, error, aside, children }: FieldProps) {
  const id = useId()
  return (
    <div>
      <div className="mb-3 flex items-center justify-between gap-4">
        <label htmlFor={id} className="text-sm font-medium text-zinc-800 dark:text-white">
          {label}
        </label>
        {aside}
      </div>
      {description && <p className="-mt-1 mb-3 text-sm text-zinc-500 dark:text-white/60">{description}</p>}
      {children(id)}
      {error && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
    </div>
  )
}

const control =
  'block h-10 w-full rounded-lg border border-zinc-200 border-b-zinc-300/80 bg-white px-3 text-base text-zinc-700 shadow-xs placeholder:text-zinc-400 focus:outline-2 focus:-outline-offset-1 focus:outline-zinc-800/15 sm:text-sm dark:border-white/10 dark:bg-white/10 dark:text-zinc-300 dark:placeholder:text-zinc-400 dark:focus:outline-white/20'

export function Input({ className, invalid, ...rest }: ComponentProps<'input'> & { invalid?: boolean }) {
  return <input className={cn(control, invalid && 'border-red-500 dark:border-red-400', className)} {...rest} />
}

export function Select({ className, children, ...rest }: ComponentProps<'select'>) {
  return (
    <select className={cn(control, 'appearance-none bg-[url(/select-chevron.svg)] bg-[length:16px] bg-[right_0.75rem_center] bg-no-repeat pr-9', className)} {...rest}>
      {children}
    </select>
  )
}

export function Checkbox({ label, className, ...rest }: ComponentProps<'input'> & { label: ReactNode }) {
  return (
    <label className={cn('inline-flex items-center gap-3 text-sm font-medium text-zinc-800 dark:text-white', className)}>
      <input
        type="checkbox"
        className="size-4.5 rounded-[.3rem] border border-zinc-300 accent-zinc-800 shadow-xs dark:border-white/10 dark:accent-white"
        {...rest}
      />
      {label}
    </label>
  )
}
