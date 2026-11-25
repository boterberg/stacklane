import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { CheckIcon, ComputerDesktopIcon, LanguageIcon, MoonIcon, SunIcon } from '@heroicons/react/24/outline'
import type { ComponentType, SVGProps } from 'react'

import { usePreferences } from '../../lib/auth'
import { cn } from '../../lib/cn'
import { locales, setLocale, t, useLocale, type Locale } from '../../lib/i18n'
import { applyTheme, useTheme, type Theme } from '../../lib/theme'

type Option<T extends string> = {
  value: T
  label: string
  icon?: ComponentType<SVGProps<SVGSVGElement>>
}

function Segmented<T extends string>({ label, value, options, onChange }: { label: string; value: T; options: Option<T>[]; onChange: (value: T) => void }) {
  return (
    <div role="radiogroup" aria-label={label} className="flex rounded-lg bg-zinc-800/5 p-[3px] dark:bg-white/10">
      {options.map((option) => {
        const Icon = option.icon
        const active = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={option.label}
            title={option.label}
            onClick={() => onChange(option.value)}
            className={cn(
              'flex h-7 flex-1 items-center justify-center rounded-md px-2 text-xs font-medium text-zinc-500 transition-colors dark:text-white/70',
              active ? 'bg-white text-zinc-800 shadow-xs dark:bg-white/20 dark:text-white' : 'hover:text-zinc-800 dark:hover:text-white',
            )}
          >
            {Icon ? <Icon className="size-4" aria-hidden /> : option.label}
          </button>
        )
      })}
    </div>
  )
}

export function ThemeSwitch({ persist = false }: { persist?: boolean }) {
  const theme = useTheme()
  const preferences = usePreferences()
  useLocale()
  return (
    <Segmented<Theme>
      label={t('Weergave')}
      value={theme}
      options={[
        { value: 'light', label: t('Licht'), icon: SunIcon },
        { value: 'dark', label: t('Donker'), icon: MoonIcon },
        { value: 'system', label: t('Systeem'), icon: ComputerDesktopIcon },
      ]}
      onChange={(next) => {
        applyTheme(next)
        if (persist) preferences.mutate({ theme: next })
      }}
    />
  )
}

export function LanguageSwitch({ persist = false }: { persist?: boolean }) {
  const locale = useLocale()
  const preferences = usePreferences()
  return (
    <Segmented<Locale>
      label={t('Taal')}
      value={locale}
      options={locales.map((option) => ({ value: option.code, label: option.code.toUpperCase() }))}
      onChange={(next) => {
        setLocale(next)
        if (persist) preferences.mutate({ locale: next })
      }}
    />
  )
}

export function LanguageMenu({ persist = false }: { persist?: boolean }) {
  const locale = useLocale()
  const preferences = usePreferences()
  const currentName = locales.find((option) => option.code === locale)?.name ?? locale
  return (
    <Menu>
      <MenuButton
        aria-label={t('Taal kiezen')}
        className="flex h-9 items-center gap-2 rounded-lg border border-zinc-200 bg-white px-3 text-sm font-medium text-zinc-700 shadow-xs transition-colors hover:bg-zinc-50 dark:border-white/10 dark:bg-white/5 dark:text-white dark:hover:bg-white/10"
      >
        <LanguageIcon className="size-4 text-zinc-500 dark:text-white/70" aria-hidden />
        {currentName}
      </MenuButton>
      <MenuItems anchor={{ to: 'bottom end', gap: 6 }} className="z-50 min-w-44 rounded-lg border border-zinc-200 bg-white p-1 shadow-xs focus:outline-none dark:border-zinc-600 dark:bg-zinc-700">
        {locales.map((option) => (
          <MenuItem key={option.code}>
            <button
              type="button"
              lang={option.code}
              onClick={() => {
                setLocale(option.code)
                if (persist) preferences.mutate({ locale: option.code })
              }}
              className="flex w-full items-center justify-between gap-3 rounded-md px-2 py-1.5 text-sm font-medium text-zinc-800 data-focus:bg-zinc-50 dark:text-white dark:data-focus:bg-zinc-600"
            >
              {option.name}
              {option.code === locale && <CheckIcon className="size-4 text-zinc-500 dark:text-white/70" aria-hidden />}
            </button>
          </MenuItem>
        ))}
      </MenuItems>
    </Menu>
  )
}

export function PreferenceRows({ persist = false }: { persist?: boolean }) {
  useLocale()
  return (
    <div className="space-y-2 px-2 py-1.5">
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm font-medium text-zinc-800 dark:text-white">{t('Weergave')}</span>
        <div className="w-28">
          <ThemeSwitch persist={persist} />
        </div>
      </div>
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm font-medium text-zinc-800 dark:text-white">{t('Taal')}</span>
        <div className="w-28">
          <LanguageSwitch persist={persist} />
        </div>
      </div>
    </div>
  )
}
