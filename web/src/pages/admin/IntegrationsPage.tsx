import { CheckCircleIcon, ExclamationTriangleIcon, LinkIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { formatRelative } from '../../lib/format'
import { useCreateIntegration, useDeleteIntegration, useIntegrations, useTestIntegration } from '../../lib/admin'
import { t } from '../../lib/i18n'

const kindLabels: Record<string, string> = {
  pterodactyl: 'Pterodactyl',
  proxmox: 'Proxmox VE',
  openstack: 'OpenStack',
  virtfusion: 'VirtFusion',
  directadmin: 'DirectAdmin',
  plesk: 'Plesk',
}

const emptyForm = {
  kind: 'pterodactyl',
  name: '',
  base_url: '',
  application_key: '',
  client_key: '',
  token_id: '',
  token_secret: '',
  node: 'pve',
  bridge: 'vmbr0',
  storage: 'local',
  ip_from: '',
  ip_to: '',
  gateway: '',
  prefix: '24',
  dns: '1.1.1.1',
  verify_tls: false,
  images: '9000 Debian 13',
  wh_username: '',
  wh_secret: '',
  wh_ip: '',
}

function parseImages(text: string) {
  return text
    .split('\n')
    .map((line) => line.trim().match(/^(\d+)\s+(.+)$/))
    .filter((match): match is RegExpMatchArray => match !== null)
    .map((match) => ({ template: Number(match[1]), name: match[2].trim() }))
}

type IntegrationInput = {
  kind: string
  name: string
  base_url: string
  credentials: Record<string, string>
  settings?: Record<string, unknown>
}

function payload(form: typeof emptyForm): IntegrationInput {
  if (form.kind === 'directadmin') {
    return {
      kind: form.kind,
      name: form.name,
      base_url: form.base_url,
      credentials: { username: form.wh_username.trim(), login_key: form.wh_secret.trim() },
      settings: { ip: form.wh_ip.trim() },
    }
  }
  if (form.kind === 'plesk') {
    return { kind: form.kind, name: form.name, base_url: form.base_url, credentials: { api_key: form.wh_secret.trim() }, settings: { ip: form.wh_ip.trim() } }
  }
  if (form.kind === 'proxmox') {
    return {
      kind: form.kind,
      name: form.name,
      base_url: form.base_url,
      credentials: {
        token_id: form.token_id.trim(),
        token_secret: form.token_secret.trim(),
      },
      settings: {
        node: form.node.trim(),
        bridge: form.bridge.trim(),
        storage: form.storage.trim(),
        ip_from: form.ip_from.trim(),
        ip_to: form.ip_to.trim(),
        gateway: form.gateway.trim(),
        prefix: Number(form.prefix) || 24,
        dns: form.dns.trim(),
        verify_tls: form.verify_tls,
        images: parseImages(form.images),
      },
    }
  }
  return {
    kind: form.kind,
    name: form.name,
    base_url: form.base_url,
    credentials: {
      application_key: form.application_key,
      client_key: form.client_key,
    },
  }
}

function CreateIntegrationPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [form, setForm] = useState(emptyForm)
  const create = useCreateIntegration()
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function close() {
    onClose()
    create.reset()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate(payload(form), {
      onSuccess: () => {
        setForm(emptyForm)
        close()
      },
    })
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={t('Koppeling toevoegen')}
      description={t('Stacklane maakt de diensten van je klanten aan op dit platform. De sleutels worden versleuteld opgeslagen.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" type="submit" form="create-integration" disabled={create.isPending}>
            {create.isPending ? t('Verbinding testen...') : t('Toevoegen')}
          </Button>
        </>
      }
    >
      <form id="create-integration" onSubmit={submit} className="space-y-6" noValidate>
        <Field label={t('Platform')} error={errors.kind}>
          {(id) => (
            <Select id={id} value={form.kind} onChange={(event) => setForm({ ...form, kind: event.target.value })}>
              <option value="pterodactyl">{t('Pterodactyl (gameservers)')}</option>
              <option value="proxmox">{t('Proxmox VE (VPS)')}</option>
              <option value="directadmin">{t('DirectAdmin (webhosting)')}</option>
              <option value="plesk">{t('Plesk (webhosting)')}</option>
              <option value="openstack" disabled>
                {t('{name}, komt eraan', { name: 'OpenStack' })}
              </option>
              <option value="virtfusion" disabled>
                {t('{name}, komt eraan', { name: 'VirtFusion' })}
              </option>
            </Select>
          )}
        </Field>
        <Field label={t('Naam')} description={t('Alleen voor jezelf, bijvoorbeeld Gamenode Amsterdam.')} error={errors.name}>
          {(id) => <Input id={id} value={form.name} invalid={!!errors.name} onChange={(event) => setForm({ ...form, name: event.target.value })} />}
        </Field>
        <Field label={t('Adres van het panel')} error={errors.base_url}>
          {(id) => (
            <Input id={id} placeholder={form.kind === 'directadmin' ? 'https://server.jouwbedrijf.nl:2222' : form.kind === 'plesk' ? 'https://server.jouwbedrijf.nl:8443' : t('https://panel.jouwbedrijf.nl')} value={form.base_url} invalid={!!errors.base_url} onChange={(event) => setForm({ ...form, base_url: event.target.value })} />
          )}
        </Field>
        {form.kind === 'directadmin' || form.kind === 'plesk' ? (
          <>
            {form.kind === 'directadmin' && (
              <Field label={t('Gebruikersnaam')} description={t('Een beheerder of reseller in DirectAdmin.')} error={errors.credentials}>
                {(id) => <Input id={id} autoComplete="off" placeholder="admin" value={form.wh_username} onChange={(event) => setForm({ ...form, wh_username: event.target.value })} />}
              </Field>
            )}
            <Field
              label={form.kind === 'directadmin' ? t('Inlogsleutel') : t('API-sleutel')}
              description={
                form.kind === 'directadmin'
                  ? t('Login Keys in DirectAdmin. Sta CMD_API_ACCOUNT_USER, CMD_API_SELECT_USERS, CMD_API_PACKAGES_USER en CMD_API_LOGIN_KEYS toe.')
                  : t('Maak er een aan met plesk bin secret_key -c.')
              }
              error={form.kind === 'plesk' ? errors.credentials : undefined}
            >
              {(id) => <Input id={id} type="password" autoComplete="off" value={form.wh_secret} onChange={(event) => setForm({ ...form, wh_secret: event.target.value })} />}
            </Field>
            <Field label={t('IP-adres voor websites')} description={t('Het gedeelde IP-adres waarop nieuwe websites komen.')} error={errors.settings}>
              {(id) => <Input id={id} placeholder="203.0.113.20" value={form.wh_ip} onChange={(event) => setForm({ ...form, wh_ip: event.target.value })} />}
            </Field>
          </>
        ) : form.kind === 'proxmox' ? (
          <>
            <Field label={t('API-token')} description={t('Datacenter → Permissions → API Tokens, in de vorm gebruiker@realm!naam.')} error={errors.credentials}>
              {(id) => <Input id={id} autoComplete="off" placeholder="stacklane@pve!panel" value={form.token_id} onChange={(event) => setForm({ ...form, token_id: event.target.value })} />}
            </Field>
            <Field label={t('Geheim van het token')}>
              {(id) => <Input id={id} type="password" autoComplete="off" value={form.token_secret} onChange={(event) => setForm({ ...form, token_secret: event.target.value })} />}
            </Field>
            <div className="grid gap-6 sm:grid-cols-3">
              <Field label={t('Node')}>{(id) => <Input id={id} value={form.node} onChange={(event) => setForm({ ...form, node: event.target.value })} />}</Field>
              <Field label={t('Bridge')}>{(id) => <Input id={id} value={form.bridge} onChange={(event) => setForm({ ...form, bridge: event.target.value })} />}</Field>
              <Field label={t('Opslag')}>{(id) => <Input id={id} value={form.storage} onChange={(event) => setForm({ ...form, storage: event.target.value })} />}</Field>
            </div>
            <div className="grid gap-6 sm:grid-cols-2">
              <Field label={t('Eerste IP-adres')} error={errors.settings}>
                {(id) => <Input id={id} placeholder="203.0.113.10" value={form.ip_from} onChange={(event) => setForm({ ...form, ip_from: event.target.value })} />}
              </Field>
              <Field label={t('Laatste IP-adres')}>
                {(id) => <Input id={id} placeholder="203.0.113.60" value={form.ip_to} onChange={(event) => setForm({ ...form, ip_to: event.target.value })} />}
              </Field>
              <Field label={t('Gateway')}>{(id) => <Input id={id} placeholder="203.0.113.1" value={form.gateway} onChange={(event) => setForm({ ...form, gateway: event.target.value })} />}</Field>
              <Field label={t('Prefix')}>{(id) => <Input id={id} inputMode="numeric" value={form.prefix} onChange={(event) => setForm({ ...form, prefix: event.target.value })} />}</Field>
            </div>
            <Field label={t('DNS-server')}>{(id) => <Input id={id} value={form.dns} onChange={(event) => setForm({ ...form, dns: event.target.value })} />}</Field>
            <Field label={t('Besturingssystemen')} description={t('Per regel het nummer van een cloud-init-template en de naam die klanten zien.')}>
              {(id) => (
                <textarea
                  id={id}
                  rows={4}
                  value={form.images}
                  onChange={(event) => setForm({ ...form, images: event.target.value })}
                  className="block w-full rounded-lg border border-zinc-200 bg-white px-3 py-2 font-mono text-sm text-zinc-700 shadow-xs dark:border-white/10 dark:bg-white/10 dark:text-zinc-300"
                />
              )}
            </Field>
            <Checkbox label={t('Certificaat van Proxmox controleren')} checked={form.verify_tls} onChange={(event) => setForm({ ...form, verify_tls: event.target.checked })} />
          </>
        ) : (
          <>
            <Field label={t('Application API-sleutel')} description={t('Admin → Application API in Pterodactyl, met lees- en schrijfrechten.')} error={errors.credentials}>
              {(id) => (
                <Input id={id} type="password" autoComplete="off" placeholder="ptla_..." value={form.application_key} onChange={(event) => setForm({ ...form, application_key: event.target.value })} />
              )}
            </Field>
            <Field label={t('Client API-sleutel')} description={t('Account → API-sleutels van een beheerder. Hiermee bedient Stacklane console, bestanden en back-ups.')}>
              {(id) => <Input id={id} type="password" autoComplete="off" placeholder="ptlc_..." value={form.client_key} onChange={(event) => setForm({ ...form, client_key: event.target.value })} />}
            </Field>
          </>
        )}
      </form>
    </SlideOver>
  )
}

export function IntegrationsPage() {
  const { data: integrations, isPending } = useIntegrations()
  const test = useTestIntegration()
  const remove = useDeleteIntegration()
  const [creating, setCreating] = useState(false)

  return (
    <>
      <PageHeader
        title={t('Koppelingen')}
        description={t('De platformen waarop Stacklane diensten aanmaakt.')}
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            {t('Koppeling toevoegen')}
          </Button>
        }
      />
      {!isPending && (integrations ?? []).length === 0 ? (
        <EmptyState title={t('Nog geen koppelingen')} description={t('Voeg je Pterodactyl-panel toe om gameservers te kunnen leveren.')} />
      ) : (
        <ul className="space-y-3">
          {(integrations ?? []).map((integration) => (
            <li key={integration.id} className="flex flex-wrap items-center gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-5 dark:border-white/10 dark:bg-white/10">
              <LinkIcon className="size-5 text-zinc-400 dark:text-white/50" />
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium text-zinc-800 dark:text-white">
                  {integration.name} <span className="font-normal text-zinc-500 dark:text-white/60">· {kindLabels[integration.kind]}</span>
                </p>
                <p className="mt-1 flex items-center gap-2 text-sm text-zinc-500 dark:text-white/70">
                  {integration.last_error ? (
                    <>
                      <ExclamationTriangleIcon className="size-4 text-amber-500" />
                      {integration.last_error}
                    </>
                  ) : (
                    <>
                      <CheckCircleIcon className="size-4 text-green-500" />
                      {integration.base_url}
                      {integration.last_checked_at && (
                        <>
                          {' '}
                          ·{' '}
                          {t('getest {when}', {
                            when: formatRelative(integration.last_checked_at),
                          })}
                        </>
                      )}
                    </>
                  )}
                </p>
              </div>
              <Button size="sm" disabled={test.isPending} onClick={() => test.mutate(integration.id)}>
                {t('Testen')}
              </Button>
              <Button size="sm" variant="ghost" icon={TrashIcon} disabled={remove.isPending} onClick={() => remove.mutate(integration.id)} aria-label={t('Verwijderen')} />
            </li>
          ))}
        </ul>
      )}
      {remove.error && <p className="mt-4 text-sm text-red-500">{remove.error instanceof ApiError ? Object.values(remove.error.fieldErrors)[0] : remove.error.message}</p>}
      <CreateIntegrationPanel open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
