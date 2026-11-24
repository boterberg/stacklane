import { CheckIcon, ClipboardDocumentIcon, KeyIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { useApiTokens, useCreateApiToken, useDeleteApiToken, type ApiToken } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { formatDate, formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { translated } from '../../lib/labels'

const areaLabels: Record<string, string> = translated({
  account: 'Account en meldingen',
  gameservers: 'Gameservers',
  billing: 'Facturering',
  support: 'Ondersteuning',
  admin: 'Beheer',
})

const lifetimes = [
  { days: 30, label: '30 dagen' },
  { days: 90, label: '90 dagen' },
  { days: 365, label: '1 jaar' },
  { days: null, label: 'Verloopt niet' },
]

function scopeSummary(scopes: string[]) {
  return scopes
    .filter((scope) => !(scope.endsWith(':read') && scopes.includes(scope.replace(':read', ':write'))))
    .map((scope) => {
      const [area, level] = scope.split(':')
      return level === 'write' ? t('{area} beheren', { area: areaLabels[area] ?? area }) : t('{area} bekijken', { area: areaLabels[area] ?? area })
    })
    .join(', ')
}

function CreateToken({ areas, open, onClose, onCreated }: { areas: string[]; open: boolean; onClose: () => void; onCreated: (secret: string) => void }) {
  const create = useCreateApiToken()
  const [name, setName] = useState('')
  const [scopes, setScopes] = useState<string[]>([])
  const [lifetime, setLifetime] = useState('90')
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function toggle(scope: string, on: boolean) {
    const [area, level] = scope.split(':')
    let next = scopes.filter((existing) => existing !== scope)
    if (on) next = [...next, scope]
    if (on && level === 'write') next = [...new Set([...next, `${area}:read`])]
    if (!on && level === 'read') next = next.filter((existing) => existing !== `${area}:write`)
    setScopes(next)
  }

  function submit() {
    create.mutate(
      { name, scopes, expires_in_days: lifetime === 'never' ? null : Number(lifetime) },
      {
        onSuccess: (token) => {
          setName('')
          setScopes([])
          onCreated(token.secret)
        },
      },
    )
  }

  return (
    <SlideOver
      open={open}
      onClose={onClose}
      title={t('API-token maken')}
      description={t('Met een token gebruik je de API namens je account.')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" disabled={create.isPending} onClick={submit}>
            {t('Maken')}
          </Button>
        </>
      }
    >
      <div className="space-y-8">
        <Field label={t('Naam')} description={t('Zodat je de token later herkent, bijvoorbeeld de naam van je script.')} error={errors.name}>
          {(id) => <Input id={id} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>
        <Field label={t('Geldig')} error={errors.expires_in_days}>
          {(id) => (
            <Select id={id} value={lifetime} onChange={(event) => setLifetime(event.target.value)}>
              {lifetimes.map((option) => (
                <option key={option.label} value={option.days ?? 'never'}>
                  {t(option.label)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <div>
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Bevoegdheden')}</p>
          <p className="mt-2 text-sm text-zinc-500 dark:text-white/60">{t('Beheren geeft ook leesrechten. Tokens kunnen nooit andere tokens of je wachtwoord beheren.')}</p>
          {errors.scopes && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{errors.scopes}</p>}
          <ul className="mt-4 divide-y divide-zinc-200 rounded-xl border border-zinc-200 dark:divide-white/10 dark:border-white/10">
            {areas.map((area) => (
              <li key={area} className="flex flex-wrap items-center justify-between gap-4 px-4 py-3">
                <span className="text-sm font-medium text-zinc-800 dark:text-white">{areaLabels[area] ?? area}</span>
                <span className="flex gap-6">
                  <Checkbox label={t('Bekijken')} checked={scopes.includes(`${area}:read`)} onChange={(event) => toggle(`${area}:read`, event.target.checked)} />
                  <Checkbox label={t('Beheren')} checked={scopes.includes(`${area}:write`)} onChange={(event) => toggle(`${area}:write`, event.target.checked)} />
                </span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </SlideOver>
  )
}

function NewSecret({ secret, onClose }: { secret: string; onClose: () => void }) {
  const [copied, setCopied] = useState(false)
  async function copy() {
    await navigator.clipboard.writeText(secret)
    setCopied(true)
  }
  return (
    <div className="mb-6 rounded-xl border border-green-200 bg-green-50 p-5 dark:border-green-400/20 dark:bg-green-400/10">
      <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Kopieer je token nu')}</p>
      <p className="mt-1 text-sm text-zinc-600 dark:text-white/70">{t('Je ziet hem maar één keer. Raak je hem kwijt, maak dan een nieuwe.')}</p>
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <code className="min-w-0 flex-1 truncate rounded-lg border border-zinc-200 bg-white px-3 py-2 font-mono text-sm text-zinc-800 dark:border-white/10 dark:bg-zinc-900 dark:text-white">{secret}</code>
        <Button icon={copied ? CheckIcon : ClipboardDocumentIcon} onClick={copy}>
          {copied ? t('Gekopieerd') : t('Kopiëren')}
        </Button>
        <Button variant="ghost" onClick={onClose}>
          {t('Klaar')}
        </Button>
      </div>
    </div>
  )
}

function TokenRow({ token, onDelete }: { token: ApiToken; onDelete: () => void }) {
  const [now] = useState(Date.now)
  const expired = token.expires_at !== null && Date.parse(token.expires_at) < now
  return (
    <li className="flex flex-wrap items-center justify-between gap-4 py-4 first:pt-0 last:pb-0">
      <div className="flex min-w-0 items-start gap-3">
        <KeyIcon className="mt-0.5 size-5 shrink-0 text-zinc-400 dark:text-white/50" aria-hidden />
        <div className="min-w-0">
          <p className="flex flex-wrap items-center gap-2 text-sm font-medium text-zinc-800 dark:text-white">
            {token.name}
            <code className="font-mono text-xs font-normal text-zinc-500 dark:text-white/60">{token.prefix}…</code>
            {expired && <Badge color="zinc">{t('Verlopen')}</Badge>}
          </p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{scopeSummary(token.scopes)}</p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">
            {token.last_used_at ? t('Laatst gebruikt {when}', { when: formatRelative(token.last_used_at) }) : t('Nog niet gebruikt')}
            {' · '}
            {token.expires_at ? t('Geldig tot {date}', { date: formatDate(token.expires_at) }) : t('Verloopt niet')}
          </p>
        </div>
      </div>
      <Button size="sm" icon={TrashIcon} onClick={onDelete}>
        {t('Verwijderen')}
      </Button>
    </li>
  )
}

export function ApiTokensTab() {
  const { data } = useApiTokens()
  const remove = useDeleteApiToken()
  const [creating, setCreating] = useState(false)
  const [secret, setSecret] = useState<string | null>(null)
  const [deleting, setDeleting] = useState<ApiToken | null>(null)
  const tokens = data?.tokens ?? []

  return (
    <div className="max-w-4xl">
      {secret && <NewSecret secret={secret} onClose={() => setSecret(null)} />}
      <Card
        title={t('API-tokens')}
        description={t('Persoonlijke tokens om de API namens je account te gebruiken.')}
        action={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            {t('Token maken')}
          </Button>
        }
      >
        {data && tokens.length === 0 ? (
          <EmptyState bare title={t('Geen API-tokens')} description={t('Je hebt nog geen tokens gemaakt.')} />
        ) : (
          <ul className="divide-y divide-zinc-200 dark:divide-white/10">
            {tokens.map((token) => (
              <TokenRow key={token.id} token={token} onDelete={() => setDeleting(token)} />
            ))}
          </ul>
        )}
      </Card>
      {data && (
        <CreateToken
          areas={data.areas}
          open={creating}
          onClose={() => setCreating(false)}
          onCreated={(value) => {
            setCreating(false)
            setSecret(value)
          }}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        title={t('Token verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setDeleting(null)}
        onConfirm={() => deleting && remove.mutate(deleting.id, { onSettled: () => setDeleting(null) })}
      >
        {t('Scripts die {name} gebruiken, krijgen meteen geen toegang meer.', { name: deleting?.name ?? '' })}
      </ConfirmDialog>
    </div>
  )
}
