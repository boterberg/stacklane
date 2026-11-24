import { CheckCircleIcon, ExclamationTriangleIcon, LinkIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { formatRelative } from '../../lib/format'
import { useCreateIntegration, useDeleteIntegration, useIntegrations, useTestIntegration } from '../../lib/admin'

const kindLabels: Record<string, string> = {
  pterodactyl: 'Pterodactyl',
  proxmox: 'Proxmox VE',
  openstack: 'OpenStack',
  virtfusion: 'VirtFusion',
}

const emptyForm = { kind: 'pterodactyl', name: '', base_url: '', application_key: '', client_key: '' }

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
    create.mutate(
      { kind: form.kind, name: form.name, base_url: form.base_url, credentials: { application_key: form.application_key, client_key: form.client_key } },
      {
        onSuccess: () => {
          setForm(emptyForm)
          close()
        },
      },
    )
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title="Koppeling toevoegen"
      description="Stacklane maakt de diensten van je klanten aan op dit platform. De sleutels worden versleuteld opgeslagen."
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            Annuleren
          </Button>
          <Button variant="primary" type="submit" form="create-integration" disabled={create.isPending}>
            {create.isPending ? 'Verbinding testen...' : 'Toevoegen'}
          </Button>
        </>
      }
    >
      <form id="create-integration" onSubmit={submit} className="space-y-6" noValidate>
        <Field label="Platform" error={errors.kind}>
          {(id) => (
            <Select id={id} value={form.kind} onChange={(event) => setForm({ ...form, kind: event.target.value })}>
              <option value="pterodactyl">Pterodactyl (gameservers)</option>
              <option value="proxmox" disabled>
                Proxmox VE, komt eraan
              </option>
              <option value="openstack" disabled>
                OpenStack, komt eraan
              </option>
              <option value="virtfusion" disabled>
                VirtFusion, komt eraan
              </option>
            </Select>
          )}
        </Field>
        <Field label="Naam" description="Alleen voor jezelf, bijvoorbeeld Gamenode Amsterdam." error={errors.name}>
          {(id) => <Input id={id} value={form.name} invalid={!!errors.name} onChange={(event) => setForm({ ...form, name: event.target.value })} />}
        </Field>
        <Field label="Adres van het panel" error={errors.base_url}>
          {(id) => (
            <Input id={id} placeholder="https://panel.jouwbedrijf.nl" value={form.base_url} invalid={!!errors.base_url} onChange={(event) => setForm({ ...form, base_url: event.target.value })} />
          )}
        </Field>
        <Field label="Application API-sleutel" description="Admin → Application API in Pterodactyl, met lees- en schrijfrechten." error={errors.credentials}>
          {(id) => <Input id={id} type="password" autoComplete="off" placeholder="ptla_..." value={form.application_key} onChange={(event) => setForm({ ...form, application_key: event.target.value })} />}
        </Field>
        <Field label="Client API-sleutel" description="Account → API-sleutels van een beheerder. Hiermee bedient Stacklane console, bestanden en back-ups.">
          {(id) => <Input id={id} type="password" autoComplete="off" placeholder="ptlc_..." value={form.client_key} onChange={(event) => setForm({ ...form, client_key: event.target.value })} />}
        </Field>
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
        title="Koppelingen"
        description="De platformen waarop Stacklane diensten aanmaakt."
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            Koppeling toevoegen
          </Button>
        }
      />
      {!isPending && (integrations ?? []).length === 0 ? (
        <EmptyState title="Nog geen koppelingen" description="Voeg je Pterodactyl-panel toe om gameservers te kunnen leveren." />
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
                      {integration.last_checked_at && <> · getest {formatRelative(integration.last_checked_at)}</>}
                    </>
                  )}
                </p>
              </div>
              <Button size="sm" disabled={test.isPending} onClick={() => test.mutate(integration.id)}>
                Testen
              </Button>
              <Button size="sm" variant="ghost" icon={TrashIcon} disabled={remove.isPending} onClick={() => remove.mutate(integration.id)} aria-label="Verwijderen" />
            </li>
          ))}
        </ul>
      )}
      {remove.error && <p className="mt-4 text-sm text-red-500">{remove.error instanceof ApiError ? Object.values(remove.error.fieldErrors)[0] : remove.error.message}</p>}
      <CreateIntegrationPanel open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
