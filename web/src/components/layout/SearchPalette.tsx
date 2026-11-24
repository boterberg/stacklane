import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions, Dialog, DialogBackdrop, DialogPanel } from '@headlessui/react'
import { MagnifyingGlassIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useNavigate } from 'react-router'

import { flatNavigationFor, type NavLink } from '../../app/navigation'
import { t } from '../../lib/i18n'

type SearchPaletteProps = {
  open: boolean
  onClose: () => void
  isAdmin: boolean
}

export function SearchPalette({ open, onClose, isAdmin }: SearchPaletteProps) {
  const flatNavigation = flatNavigationFor(isAdmin)
  const [query, setQuery] = useState('')
  const navigate = useNavigate()
  const normalized = query.trim().toLowerCase()
  const results = normalized === '' ? flatNavigation : flatNavigation.filter((item) => t(item.label).toLowerCase().includes(normalized))

  function close() {
    onClose()
    setQuery('')
  }

  function select(item: NavLink | null) {
    if (!item) return
    close()
    if (item.external) {
      window.open(item.to, '_blank', 'noreferrer')
    } else {
      navigate(item.to)
    }
  }

  return (
    <Dialog open={open} onClose={close} className="relative z-50">
      <DialogBackdrop className="fixed inset-0 bg-zinc-900/20 dark:bg-black/30" />
      <div className="fixed inset-0 overflow-y-auto px-4 pt-[12vh]">
        <DialogPanel className="mx-auto max-w-lg overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-lg dark:border-zinc-600 dark:bg-zinc-700">
          <Combobox onChange={select}>
            <div className="relative flex h-12 items-center border-b border-zinc-200 dark:border-zinc-600">
              <MagnifyingGlassIcon className="pointer-events-none absolute left-4 size-5 text-zinc-400" aria-hidden />
              <ComboboxInput
                autoFocus
                className="h-full w-full bg-transparent pr-12 pl-11 text-base text-zinc-800 placeholder:text-zinc-400 focus:outline-none sm:text-sm dark:text-white"
                placeholder={t('Zoeken...')}
                onChange={(event) => setQuery(event.target.value)}
              />
              <button type="button" onClick={close} className="absolute right-3 rounded-md p-1 text-zinc-400 hover:text-zinc-800 dark:hover:text-white" aria-label={t('Sluiten')}>
                <XMarkIcon className="size-4" />
              </button>
            </div>
            {results.length > 0 ? (
              <ComboboxOptions static className="max-h-80 overflow-y-auto p-1.5">
                {results.map((item) => {
                  const Icon = item.icon
                  return (
                    <ComboboxOption
                      key={item.label}
                      value={item}
                      className="flex cursor-pointer items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-zinc-800 data-focus:bg-zinc-100 dark:text-white dark:data-focus:bg-zinc-600"
                    >
                      <Icon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
                      {t(item.label)}
                    </ComboboxOption>
                  )
                })}
              </ComboboxOptions>
            ) : (
              <p className="px-4 py-3.5 text-center text-sm text-zinc-500 dark:text-zinc-400">{t('Geen resultaten gevonden')}</p>
            )}
          </Combobox>
        </DialogPanel>
      </div>
    </Dialog>
  )
}
