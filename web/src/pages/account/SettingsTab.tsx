import { ComputerDesktopIcon, MoonIcon, SunIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Field, Select } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { useMe, usePreferences, type Profile } from '../../lib/auth'
import { cn } from '../../lib/cn'
import { locales, setLocale, t, useLocale, type Locale } from '../../lib/i18n'
import { applyTheme, useTheme, type Theme } from '../../lib/theme'

const themes = [
  { value: 'light' as const, label: 'Licht', icon: SunIcon },
  { value: 'dark' as const, label: 'Donker', icon: MoonIcon },
  { value: 'system' as const, label: 'Systeem', icon: ComputerDesktopIcon },
]

function timezones() {
  const supported = (Intl as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf?.('timeZone') ?? []
  return supported.length > 0 ? supported : ['Europe/Amsterdam', 'Europe/Brussels', 'UTC']
}

function Appearance() {
  const theme = useTheme()
  const preferences = usePreferences()
  function choose(next: Theme) {
    applyTheme(next)
    preferences.mutate({ theme: next })
  }
  return (
    <Card title={t('Uiterlijk')} description={t('Kies hoe het panel er voor jou uitziet.')}>
      <div role="radiogroup" aria-label={t('Uiterlijk')} className="grid grid-cols-3 rounded-lg bg-zinc-800/5 p-[3px] dark:bg-white/10">
        {themes.map((option) => (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={theme === option.value}
            onClick={() => choose(option.value)}
            className={cn(
              'flex h-9 items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors',
              theme === option.value ? 'bg-white text-zinc-800 shadow-xs dark:bg-white/20 dark:text-white' : 'text-zinc-500 hover:text-zinc-800 dark:text-white/70 dark:hover:text-white',
            )}
          >
            <option.icon className="size-4.5" aria-hidden />
            {t(option.label)}
          </button>
        ))}
      </div>
    </Card>
  )
}

function Region({ profile }: { profile: Profile }) {
  const current = useLocale()
  const preferences = usePreferences()
  const [locale, setChosenLocale] = useState<Locale>(current)
  const [timezone, setTimezone] = useState(profile.timezone)
  const errors = preferences.error instanceof ApiError ? preferences.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    preferences.mutate({ locale, timezone }, { onSuccess: () => setLocale(locale) })
  }

  return (
    <form onSubmit={submit} noValidate>
      <Card
        title={t('Taal en regio')}
        description={t('De taal en tijdzone van je account.')}
        footer={
          <>
            {preferences.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span>}
            <Button type="submit" variant="primary" disabled={preferences.isPending}>
              {t('Wijzigingen opslaan')}
            </Button>
          </>
        }
      >
        <div className="space-y-6">
          <Field label={t('Taal')} description={t('Voor het panel en de e-mails die we je sturen.')} error={errors.locale}>
            {(id) => (
              <Select id={id} value={locale} onChange={(event) => setChosenLocale(event.target.value as Locale)}>
                {locales.map((option) => (
                  <option key={option.code} value={option.code}>
                    {option.name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t('Tijdzone')} description={t('Tijden in het panel volgen deze tijdzone.')} error={errors.timezone}>
            {(id) => (
              <Select id={id} value={timezone} onChange={(event) => setTimezone(event.target.value)}>
                {timezones().map((zone) => (
                  <option key={zone} value={zone}>
                    {zone}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>
      </Card>
    </form>
  )
}

export function SettingsTab() {
  const { data: profile } = useMe()
  return (
    <div className="max-w-xl space-y-6">
      <Appearance />
      {profile && <Region profile={profile} />}
    </div>
  )
}
