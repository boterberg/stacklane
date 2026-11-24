import { CalendarIcon, EyeIcon, LockClosedIcon, LockOpenIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link, useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input } from '../../components/ui/Field'
import { Tabs } from '../../components/ui/Tabs'
import { ApiError } from '../../lib/api'
import { holderName, useAuthCode, useDomain, useSetLock, useSetNameservers, useWhoisProfiles, type Domain, type Registry } from '../../lib/domains'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'

type Tab = 'overview' | 'nameservers' | 'transfer'

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="py-3 first:pt-0 last:pb-0">
      <dt className="text-sm font-medium text-zinc-800 dark:text-white">{label}</dt>
      <dd className="mt-1 text-sm text-zinc-500 dark:text-white/70">{children}</dd>
    </div>
  )
}

function Overview({ domain, registry }: { domain: Domain; registry: Registry | null }) {
  const { data: profiles = [] } = useWhoisProfiles()
  const profile = profiles.find((option) => option.id === domain.details.whois_profile_id)
  const expires = registry?.expires_on ?? domain.details.expires_on
  return (
    <div className="grid max-w-5xl gap-6 lg:grid-cols-2">
      <Card title={t('Domein')}>
        <dl className="divide-y divide-zinc-200 dark:divide-white/10">
          <Row label={t('Verloopt op')}>{expires ? formatDate(expires) : '-'}</Row>
          <Row label={t('Verlenging')}>
            {domain.cancelled ? t('Wordt niet verlengd') : t('Elk jaar automatisch, via abonnement #{code}', { code: domain.subscription_code ?? '-' })}
          </Row>
          <Row label={t('Houder')}>
            {profile ? (
              <Link to="/domains/whois-profiles" className="text-zinc-800 underline-offset-4 hover:underline dark:text-white">
                {holderName(profile)}
              </Link>
            ) : (
              '-'
            )}
          </Row>
          <Row label={t('Verhuisvergrendeling')}>{registry ? (registry.locked ? t('Aan') : t('Uit')) : '-'}</Row>
        </dl>
      </Card>
      <Card title={t('Nameservers')}>
        <ul className="space-y-1 font-mono text-sm text-zinc-700 dark:text-zinc-300">
          {(registry?.nameservers ?? domain.details.nameservers ?? []).map((name) => (
            <li key={name}>{name}</li>
          ))}
        </ul>
      </Card>
    </div>
  )
}

function Nameservers({ domain, registry }: { domain: Domain; registry: Registry | null }) {
  const current = registry?.nameservers ?? domain.details.nameservers ?? []
  const [values, setValues] = useState(() => [...current, '', ''].slice(0, Math.max(current.length, 2)))
  const save = useSetNameservers(domain.id)
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  return (
    <div className="max-w-2xl">
      <Card
        title={t('Nameservers')}
        description={t('Wijzig je nameservers alleen als je DNS ergens anders beheert. Het kan tot 24 uur duren voor de wijziging overal zichtbaar is.')}
        footer={
          <>
            {save.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span>}
            <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate(values.filter((value) => value.trim()))}>
              {t('Wijzigingen opslaan')}
            </Button>
          </>
        }
      >
        <div className="space-y-4">
          {values.map((value, index) => (
            <Field key={index} label={t('Nameserver {number}', { number: index + 1 })}>
              {(id) => (
                <Input
                  id={id}
                  placeholder={`ns${index + 1}.voorbeeld.nl`}
                  value={value}
                  invalid={!!errors.nameservers}
                  onChange={(event) => setValues(values.map((current, position) => (position === index ? event.target.value : current)))}
                />
              )}
            </Field>
          ))}
          {values.length < 6 && (
            <Button size="sm" onClick={() => setValues([...values, ''])}>
              {t('Nameserver toevoegen')}
            </Button>
          )}
          {(errors.nameservers || errors.domain) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.nameservers ?? errors.domain}</p>}
        </div>
      </Card>
    </div>
  )
}

function Transfer({ domain, registry }: { domain: Domain; registry: Registry | null }) {
  const code = useAuthCode(domain.id)
  const lock = useSetLock(domain.id)
  const locked = registry?.locked ?? false
  const error = code.error ?? lock.error
  return (
    <div className="max-w-2xl space-y-6">
      <Card title={t('Verhuisvergrendeling')} description={t('Zolang de vergrendeling aan staat, kan niemand het domein naar een andere provider verhuizen.')}>
        <Button icon={locked ? LockOpenIcon : LockClosedIcon} disabled={lock.isPending || !registry} onClick={() => lock.mutate(!locked)}>
          {locked ? t('Vergrendeling uitzetten') : t('Vergrendeling aanzetten')}
        </Button>
      </Card>
      <Card title={t('Autorisatiecode')} description={t('Deze code heb je nodig om het domein naar een andere provider te verhuizen. Deel hem met niemand anders.')}>
        {code.data ? (
          <code className="block font-mono text-sm text-zinc-800 dark:text-white">{code.data.auth_code}</code>
        ) : (
          <Button icon={EyeIcon} disabled={code.isPending} onClick={() => code.mutate()}>
            {t('Autorisatiecode tonen')}
          </Button>
        )}
      </Card>
      {error instanceof ApiError && <p className="text-sm font-medium text-red-500 dark:text-red-400">{error.fieldErrors.domain ?? error.message}</p>}
    </div>
  )
}

export function DomainPage() {
  const { id = '' } = useParams()
  const { data: domain, isPending, error } = useDomain(id)
  const [tab, setTab] = useState<Tab>('overview')

  if (isPending) return null
  if (error || !domain) {
    return (
      <EmptyState
        title={t('Domein niet gevonden')}
        description={t('Dit domein bestaat niet of is van iemand anders.')}
        action={
          <ButtonLink to="/domains" size="sm">
            {t('Naar domeinen')}
          </ButtonLink>
        }
      />
    )
  }

  const expires = domain.registry?.expires_on ?? domain.details.expires_on
  return (
    <>
      <div className="mb-6">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{domain.name}</h1>
          <Badge color={serviceStatusColors[domain.status]}>{serviceStatusLabels[domain.status]}</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
          {expires && (
            <span className="flex items-center gap-2">
              <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('Verloopt op {date}', { date: formatDate(expires) })}
            </span>
          )}
          {domain.details.nameservers?.[0] && (
            <span className="flex items-center gap-2">
              <ServerStackIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {domain.details.nameservers[0]}
            </span>
          )}
        </div>
      </div>

      {domain.status !== 'active' ? (
        <EmptyState
          title={domain.status === 'failed' ? t('Er ging iets mis') : domain.details.operation === 'transfer' ? t('Je domein wordt verhuisd') : t('Je domein wordt geregistreerd')}
          description={
            domain.status === 'failed'
              ? t('We konden je domein niet registreren. Ons team is op de hoogte.')
              : domain.status === 'pending'
                ? t('Zodra de factuur betaald is, gaan we aan de slag.')
                : t('Dit duurt meestal een paar minuten. Een verhuizing kan een paar dagen duren.')
          }
        />
      ) : (
        <>
          <Tabs
            tabs={[
              { id: 'overview' as const, label: 'Overzicht' },
              { id: 'nameservers' as const, label: 'Nameservers' },
              { id: 'transfer' as const, label: 'Verhuizen' },
            ]}
            active={tab}
            onChange={setTab}
          />
          {tab === 'overview' && <Overview domain={domain} registry={domain.registry} />}
          {tab === 'nameservers' && <Nameservers key={domain.registry?.nameservers.join() ?? ''} domain={domain} registry={domain.registry} />}
          {tab === 'transfer' && <Transfer domain={domain} registry={domain.registry} />}
        </>
      )}
    </>
  )
}
