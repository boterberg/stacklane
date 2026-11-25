import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from '@headlessui/react'
import { ChevronDownIcon, MagnifyingGlassIcon, Squares2X2Icon, ListBulletIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { CheckIcon } from '@heroicons/react/20/solid'
import type { ReactNode } from 'react'

import { cn } from '../../lib/cn'
import { t } from '../../lib/i18n'

export type ListView = 'grid' | 'table'

const controlButton =
  'flex h-8 items-center gap-2 rounded-md border border-zinc-200 border-b-zinc-300/80 bg-white px-3 text-sm text-zinc-700 shadow-xs dark:border-white/10 dark:bg-white/10 dark:text-zinc-200 dark:shadow-none'

const options =
  'z-50 min-w-(--button-width) rounded-lg border border-zinc-200 bg-white p-1 shadow-lg focus:outline-none dark:border-zinc-600 dark:bg-zinc-700'

const option =
  'group flex cursor-pointer items-center gap-2 rounded-md py-1.5 pr-2 pl-2 text-sm text-zinc-800 data-focus:bg-zinc-100 dark:text-white dark:data-focus:bg-zinc-600'

export function ListToolbar({ children }: { children: ReactNode }) {
  return <div className="mb-6 flex flex-wrap items-center gap-2">{children}</div>
}

export function ToolbarSpacer() {
  return <div className="flex-1" />
}

export function SearchField({ value, onChange, placeholder }: { value: string; onChange: (value: string) => void; placeholder: string }) {
  return (
    <div className="relative w-full sm:w-72">
      <MagnifyingGlassIcon className="pointer-events-none absolute top-[7px] left-3.5 size-[18px] text-zinc-400 dark:text-white/60" aria-hidden />
      <input
        type="search"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="accent-focus h-8 w-full rounded-md border border-zinc-200 border-b-zinc-300/80 bg-white pr-3 pl-10 text-sm text-zinc-700 shadow-xs placeholder:text-zinc-400 focus:outline-2 focus:-outline-offset-1 focus:outline-zinc-800/15 dark:border-white/10 dark:bg-white/10 dark:text-zinc-200 dark:shadow-none dark:placeholder:text-white/60 dark:focus:outline-white/20"
      />
    </div>
  )
}

type MultiSelectProps<T extends string> = {
  values: T[]
  options: T[]
  labels: Record<T, string>
  onChange: (values: T[]) => void
  noun: [string, string]
  placeholder: string
}

export function MultiSelectFilter<T extends string>({ values, options: choices, labels, onChange, noun, placeholder }: MultiSelectProps<T>) {
  const summary = values.length === 0 ? placeholder : values.length === 1 ? labels[values[0]] : t('{count} {noun} geselecteerd', { count: values.length, noun: noun[1] })
  return (
    <Listbox value={values} onChange={onChange} multiple>
      <div className="relative">
        <ListboxButton className={cn(controlButton, 'min-w-64', values.length > 0 ? 'pr-16' : 'pr-9', values.length === 0 && 'text-zinc-400 dark:text-white/60')}>
          {summary}
          <ChevronDownIcon className="pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-zinc-400" aria-hidden />
        </ListboxButton>
        {values.length > 0 && (
          <button
            type="button"
            onClick={() => onChange([])}
            aria-label={t('Filter wissen')}
            className="absolute top-1/2 right-[30px] -translate-y-1/2 rounded p-0.5 text-zinc-400 hover:text-zinc-800 dark:text-white/60 dark:hover:text-white"
          >
            <XMarkIcon className="size-3.5" />
          </button>
        )}
      </div>
      <ListboxOptions anchor={{ to: 'bottom start', gap: 4 }} className={options}>
        {choices.map((choice) => (
          <ListboxOption key={choice} value={choice} className={option}>
            <span className="flex size-4 items-center justify-center rounded-[.3rem] border border-zinc-300 bg-white group-data-selected:border-zinc-800 group-data-selected:bg-zinc-800 dark:border-white/20 dark:bg-white/10 dark:group-data-selected:border-white dark:group-data-selected:bg-white">
              <CheckIcon className="hidden size-3 text-white group-data-selected:block dark:text-zinc-800" aria-hidden />
            </span>
            {labels[choice]}
          </ListboxOption>
        ))}
      </ListboxOptions>
    </Listbox>
  )
}

export function PerPageSelect({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  return (
    <Listbox value={value} onChange={onChange}>
      <ListboxButton className={controlButton}>
        {t('{count} per pagina', { count: value })}
        <ChevronDownIcon className="size-4 text-zinc-400" aria-hidden />
      </ListboxButton>
      <ListboxOptions anchor={{ to: 'bottom end', gap: 4 }} className={options}>
        {[5, 10, 25, 50].map((size) => (
          <ListboxOption key={size} value={size} className={option}>
            <CheckIcon className="invisible size-4 group-data-selected:visible" aria-hidden />
            {t('{count} per pagina', { count: size })}
          </ListboxOption>
        ))}
      </ListboxOptions>
    </Listbox>
  )
}

export function ViewToggle({ value, onChange }: { value: ListView; onChange: (value: ListView) => void }) {
  const button = (view: ListView, label: string, Icon: typeof Squares2X2Icon) => (
    <button
      type="button"
      aria-label={label}
      aria-pressed={value === view}
      onClick={() => onChange(view)}
      className={cn(
        'flex h-7 w-11 items-center justify-center rounded-md text-zinc-500 transition-colors dark:text-white/70',
        value === view ? 'bg-white text-zinc-800 shadow-xs dark:bg-white/20 dark:text-white' : 'hover:text-zinc-800 dark:hover:text-white',
      )}
    >
      <Icon className="size-[18px]" aria-hidden />
    </button>
  )
  return (
    <div className="flex rounded-lg bg-zinc-800/5 p-[3px] dark:bg-white/10">
      {button('grid', t('Kaartweergave'), Squares2X2Icon)}
      {button('table', t('Lijstweergave'), ListBulletIcon)}
    </div>
  )
}
