import { PencilSquareIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { useDeleteDomainExtension, useDomainExtensions, useIntegrations, useSaveDomainExtension, type DomainExtension } from '../../lib/admin'
import { ApiError } from '../../lib/api'
import { formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'

function euros(cents: number) {
  return (cents / 100).toFixed(2).replace('.', ',')
}

function cents(value: string) {
  return Math.round(Number(value.replace(',', '.')) * 100)
}

function ExtensionPanel({ extension, onClose }: { extension: DomainExtension | null; onClose: () => void }) {
  const { data: integrations = [] } = useIntegrations()
  const registrars = integrations.filter((integration) => integration.kind === 'openprovider')
  const save = useSaveDomainExtension()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const [form, setForm] = useState({
    integration_id: extension?.integration_id ?? registrars[0]?.id ?? '',
    extension: extension?.extension ?? '',
    register: extension ? euros(extension.register_cents) : '',
    renew: extension ? euros(extension.renew_cents) : '',
    transfer: extension ? euros(extension.transfer_cents) : '',
    is_active: extension?.is_active ?? true,
  })

  function submit() {
    save.mutate(
      {
        id: extension?.id,
        integration_id: form.integration_id || registrars[0]?.id || '',
        extension: form.extension,
        register_cents: cents(form.register),
        renew_cents: cents(form.renew),
        transfer_cents: cents(form.transfer),
        is_active: form.is_active,
      },
      { onSuccess: onClose },
    )
  }

  return (
    <SlideOver
      open
      onClose={onClose}
      title={extension ? t('Extensie bewerken') : t('Extensie toevoegen')}
      description={t('Prijzen zijn per jaar en inclusief btw. Klanten zien alleen actieve extensies.')}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" disabled={save.isPending} onClick={submit}>
            {extension ? t('Opslaan') : t('Toevoegen')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        <Field label={t('Registrar')} error={errors.integration_id}>
          {(id) => (
            <Select id={id} value={form.integration_id} onChange={(event) => setForm({ ...form, integration_id: event.target.value })}>
              {registrars.length === 0 && <option value="">{t('Koppel eerst Openprovider')}</option>}
              {registrars.map((integration) => (
                <option key={integration.id} value={integration.id}>
                  {integration.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('Extensie')} error={errors.extension}>
          {(id) => <Input id={id} placeholder="nl" invalid={!!errors.extension} value={form.extension} onChange={(event) => setForm({ ...form, extension: event.target.value })} />}
        </Field>
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-3">
          <Field label={t('Registreren')} error={errors.register_cents}>
            {(id) => <Input id={id} inputMode="decimal" placeholder="7,99" value={form.register} onChange={(event) => setForm({ ...form, register: event.target.value })} />}
          </Field>
          <Field label={t('Verlengen')} error={errors.renew_cents}>
            {(id) => <Input id={id} inputMode="decimal" placeholder="8,99" value={form.renew} onChange={(event) => setForm({ ...form, renew: event.target.value })} />}
          </Field>
          <Field label={t('Verhuizen')} error={errors.transfer_cents}>
            {(id) => <Input id={id} inputMode="decimal" placeholder="0,00" value={form.transfer} onChange={(event) => setForm({ ...form, transfer: event.target.value })} />}
          </Field>
        </div>
        <Checkbox label={t('Klanten kunnen deze extensie bestellen')} checked={form.is_active} onChange={(event) => setForm({ ...form, is_active: event.target.checked })} />
      </div>
    </SlideOver>
  )
}

export function DomainExtensionsPage() {
  const { data: extensions, isPending } = useDomainExtensions()
  const remove = useDeleteDomainExtension()
  const [editing, setEditing] = useState<DomainExtension | 'new' | null>(null)
  const [deleting, setDeleting] = useState<DomainExtension | null>(null)

  return (
    <>
      <PageHeader
        title={t('Domeinextensies')}
        description={t('Welke extensies klanten kunnen registreren, en wat ze kosten.')}
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setEditing('new')}>
            {t('Extensie toevoegen')}
          </Button>
        }
      />
      {!isPending && (extensions ?? []).length === 0 ? (
        <EmptyState title={t('Geen extensies')} description={t('Voeg een extensie toe, zoals .nl of .com, om domeinen te verkopen.')} />
      ) : (
        <div className="overflow-x-auto rounded-xl border border-zinc-200 bg-white dark:border-white/10 dark:bg-white/10">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500 dark:border-white/10 dark:text-white/60">
              <tr>
                <th className="px-6 py-3 font-medium">{t('Extensie')}</th>
                <th className="px-6 py-3 font-medium">{t('Registreren')}</th>
                <th className="px-6 py-3 font-medium">{t('Verlengen')}</th>
                <th className="px-6 py-3 font-medium">{t('Verhuizen')}</th>
                <th className="px-6 py-3 font-medium">{t('Domeinen')}</th>
                <th className="px-6 py-3" />
              </tr>
            </thead>
            <tbody className="divide-y divide-zinc-200 dark:divide-white/10">
              {(extensions ?? []).map((extension) => (
                <tr key={extension.id}>
                  <td className="px-6 py-4">
                    <span className="font-medium text-zinc-800 dark:text-white">.{extension.extension}</span>
                    {!extension.is_active && (
                      <Badge color="zinc" className="ml-3">
                        {t('Niet te bestellen')}
                      </Badge>
                    )}
                    <p className="text-zinc-500 dark:text-white/60">{extension.integration_name}</p>
                  </td>
                  <td className="px-6 py-4 text-zinc-800 dark:text-white">{formatEuro(extension.register_cents)}</td>
                  <td className="px-6 py-4 text-zinc-800 dark:text-white">{formatEuro(extension.renew_cents)}</td>
                  <td className="px-6 py-4 text-zinc-800 dark:text-white">{formatEuro(extension.transfer_cents)}</td>
                  <td className="px-6 py-4 text-zinc-800 dark:text-white">{extension.domains}</td>
                  <td className="px-6 py-4">
                    <div className="flex justify-end gap-2">
                      <Button size="sm" icon={PencilSquareIcon} onClick={() => setEditing(extension)}>
                        {t('Bewerken')}
                      </Button>
                      <Button size="sm" icon={TrashIcon} disabled={extension.domains > 0} onClick={() => setDeleting(extension)}>
                        {t('Verwijderen')}
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {editing !== null && <ExtensionPanel key={editing === 'new' ? 'new' : editing.id} extension={editing === 'new' ? null : editing} onClose={() => setEditing(null)} />}
      <ConfirmDialog
        open={deleting !== null}
        title={t('Extensie verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setDeleting(null)}
        onConfirm={() => deleting && remove.mutate(deleting.id, { onSettled: () => setDeleting(null) })}
      >
        {t('Klanten kunnen deze extensie daarna niet meer bestellen.')}
      </ConfirmDialog>
    </>
  )
}
