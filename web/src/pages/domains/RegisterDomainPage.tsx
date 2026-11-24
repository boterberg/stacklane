import { CheckCircleIcon, MagnifyingGlassIcon, XCircleIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { Tabs } from '../../components/ui/Tabs'
import { ApiError } from '../../lib/api'
import { holderName, useDomainSearch, useOrderDomain, useWhoisProfiles, type SearchResult } from '../../lib/domains'
import { formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'

type Mode = 'register' | 'transfer'

export function RegisterDomainPage() {
  const [mode, setMode] = useState<Mode>('register')
  const [input, setInput] = useState('')
  const [query, setQuery] = useState('')
  const [chosen, setChosen] = useState<SearchResult | null>(null)
  const [profileId, setProfileId] = useState('')
  const [authCode, setAuthCode] = useState('')
  const search = useDomainSearch(query)
  const { data: profiles = [] } = useWhoisProfiles()
  const order = useOrderDomain()
  const navigate = useNavigate()
  const errors = order.error instanceof ApiError ? order.error.fieldErrors : {}
  const searchError = search.error instanceof ApiError ? (search.error.fieldErrors.query ?? search.error.message) : null
  const profile = profileId || profiles[0]?.id || ''
  const transfer = mode === 'transfer'

  function submitSearch(event: FormEvent) {
    event.preventDefault()
    setChosen(null)
    order.reset()
    setQuery(input.trim())
  }

  function place() {
    if (!chosen) return
    order.mutate({ domain: chosen.domain, whois_profile_id: profile, transfer, auth_code: transfer ? authCode : undefined }, { onSuccess: (placed) => navigate(`/billing/invoices/${placed.invoice_id}`) })
  }

  const results = (search.data ?? []).filter((result) => !transfer || result === search.data?.[0])

  return (
    <>
      <PageHeader title={transfer ? t('Domein verhuizen') : t('Domein registreren')} description={t('Zoek een naam. Na betaling regelen wij de registratie of verhuizing.')} />
      <Tabs
        tabs={[
          { id: 'register' as const, label: 'Registreren' },
          { id: 'transfer' as const, label: 'Verhuizen' },
        ]}
        active={mode}
        onChange={(next) => {
          setMode(next)
          setChosen(null)
          order.reset()
        }}
      />
      <div className="grid max-w-5xl gap-6 lg:grid-cols-[1fr_20rem]">
        <div className="space-y-6">
          <form onSubmit={submitSearch} className="flex gap-3">
            <div className="flex-1">
              <Input
                icon={MagnifyingGlassIcon}
                aria-label={t('Domeinnaam')}
                placeholder={transfer ? t('jouwdomein.nl') : t('jouwbedrijf of jouwbedrijf.nl')}
                value={input}
                onChange={(event) => setInput(event.target.value)}
              />
            </div>
            <Button type="submit" variant="primary" disabled={!input.trim() || search.isFetching}>
              {search.isFetching ? t('Zoeken...') : t('Zoeken')}
            </Button>
          </form>
          {searchError && <p className="text-sm font-medium text-red-500 dark:text-red-400">{searchError}</p>}
          {results.length > 0 && (
            <ul className="divide-y divide-zinc-200 rounded-xl border border-zinc-200 bg-white dark:divide-white/10 dark:border-white/10 dark:bg-white/10">
              {results.map((result) => {
                const orderable = !result.premium && (transfer ? !result.available : result.available)
                const price = transfer ? result.transfer_cents : result.register_cents
                const selected = chosen?.domain === result.domain
                return (
                  <li key={result.domain} className="flex flex-wrap items-center justify-between gap-4 px-6 py-4">
                    <div className="flex items-center gap-3">
                      {orderable ? (
                        <CheckCircleIcon className="size-5 text-green-600 dark:text-green-400" aria-hidden />
                      ) : (
                        <XCircleIcon className="size-5 text-zinc-400 dark:text-white/40" aria-hidden />
                      )}
                      <div>
                        <p className="text-sm font-medium text-zinc-800 dark:text-white">{result.domain}</p>
                        <p className="text-sm text-zinc-500 dark:text-white/60">
                          {result.premium
                            ? t('Premium domein, neem contact op')
                            : transfer
                              ? result.available
                                ? t('Dit domein is nog vrij')
                                : t('Kan verhuisd worden')
                              : result.available
                                ? t('Beschikbaar')
                                : t('Bezet')}
                        </p>
                      </div>
                    </div>
                    {orderable && (
                      <div className="flex items-center gap-4">
                        <div className="text-right text-sm">
                          <p className="font-medium text-zinc-800 dark:text-white">{formatEuro(price)}</p>
                          <p className="text-zinc-500 dark:text-white/60">{t('daarna {price} per jaar', { price: formatEuro(result.renew_cents) })}</p>
                        </div>
                        <Button size="sm" variant={selected ? 'primary' : undefined} onClick={() => setChosen(result)}>
                          {selected ? t('Gekozen') : t('Kiezen')}
                        </Button>
                      </div>
                    )}
                  </li>
                )
              })}
            </ul>
          )}
        </div>
        <div>
          <Card title={t('Overzicht')}>
            {chosen ? (
              <div className="space-y-6">
                <div className="text-sm">
                  <p className="font-medium text-zinc-800 dark:text-white">{chosen.domain}</p>
                  <p className="mt-1 text-zinc-500 dark:text-white/60">
                    {t('{price} voor het eerste jaar', { price: formatEuro(transfer ? chosen.transfer_cents : chosen.register_cents) })}
                  </p>
                </div>
                {profiles.length === 0 ? (
                  <p className="text-sm text-zinc-500 dark:text-white/60">
                    {t('Voeg eerst een WHOIS-profiel toe met de gegevens van de houder.')}{' '}
                    <Link to="/domains/whois-profiles" className="font-medium text-zinc-800 underline-offset-4 hover:underline dark:text-white">
                      {t('Profiel toevoegen')}
                    </Link>
                  </p>
                ) : (
                  <Field label={t('Houder')} error={errors.whois_profile_id}>
                    {(id) => (
                      <Select id={id} value={profile} onChange={(event) => setProfileId(event.target.value)}>
                        {profiles.map((option) => (
                          <option key={option.id} value={option.id}>
                            {holderName(option)}
                          </option>
                        ))}
                      </Select>
                    )}
                  </Field>
                )}
                {transfer && (
                  <Field label={t('Autorisatiecode')} description={t('Vraag deze op bij je huidige provider.')} error={errors.auth_code}>
                    {(id) => <Input id={id} autoComplete="off" value={authCode} onChange={(event) => setAuthCode(event.target.value)} />}
                  </Field>
                )}
                {errors.domain && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.domain}</p>}
                <Button variant="primary" className="w-full" disabled={!profile || order.isPending} onClick={place}>
                  {t('Bestellen en betalen')}
                </Button>
              </div>
            ) : (
              <p className="text-sm text-zinc-500 dark:text-white/60">{t('Kies een domein uit de zoekresultaten.')}</p>
            )}
          </Card>
        </div>
      </div>
    </>
  )
}
