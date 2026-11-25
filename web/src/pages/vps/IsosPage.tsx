import { CalendarIcon, CpuChipIcon, PlayCircleIcon, PlusIcon, ServerStackIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { Modal } from '../../components/ui/Modal'
import { DataTable, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useDeleteIso, useImportIso, useIsos, type Iso } from '../../lib/isos'

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

export function IsoStatus({ iso }: { iso: Iso }) {
  if (iso.status === 'importing') return <Badge color="blue">{t('Importeren')}</Badge>
  if (iso.status === 'failed') return <Badge color="red">{t('Mislukt')}</Badge>
  return iso.mounted_on ? <Badge color="green">{t('Gekoppeld')}</Badge> : <Badge>{t('Klaar')}</Badge>
}

function ImportIso({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [name, setName] = useState('')
  const [url, setUrl] = useState('')
  const [firmware, setFirmware] = useState<'bios' | 'uefi'>('bios')
  const create = useImportIso()
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function close() {
    create.reset()
    setName('')
    setUrl('')
    setFirmware('bios')
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate({ name, url, firmware }, { onSuccess: close })
  }

  return (
    <Modal
      open={open}
      onClose={close}
      title={t('ISO importeren')}
      description={t('Importeer een ISO via een publieke link.')}
      footer={
        <>
          <p className="text-sm text-zinc-500 dark:text-white/70">{t("Ongebruikte ISO's worden na 30 dagen verwijderd.")}</p>
          <Button variant="primary" type="submit" form="import-iso" disabled={create.isPending}>
            {create.isPending ? t('Bezig...') : t('Importeren')}
          </Button>
        </>
      }
    >
      <form id="import-iso" onSubmit={submit} className="space-y-5" noValidate>
        <Field label={t('Naam')} error={errors.name}>
          {(id) => <Input id={id} placeholder={t('bijv. Ubuntu 24.04 installer')} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>
        <Field label={t('ISO-URL')} error={errors.url}>
          {(id) => <Input id={id} type="url" placeholder="https://example.com/image.iso" invalid={!!errors.url} value={url} onChange={(event) => setUrl(event.target.value)} />}
        </Field>
        <fieldset>
          <legend className="text-sm font-medium text-zinc-800 dark:text-white">{t('Firmware')}</legend>
          <div className="mt-3 flex gap-6">
            {(['bios', 'uefi'] as const).map((option) => (
              <label key={option} className="inline-flex items-center gap-2 text-sm text-zinc-800 dark:text-white">
                <input type="radio" name="firmware" className="size-4 accent-zinc-800 dark:accent-white" checked={firmware === option} onChange={() => setFirmware(option)} />
                {option === 'bios' ? 'BIOS' : 'UEFI'}
              </label>
            ))}
          </div>
        </fieldset>
      </form>
    </Modal>
  )
}

export function IsosPage() {
  const { data, isPending } = useIsos()
  const remove = useDeleteIso()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [importing, setImporting] = useState(false)
  const [confirm, setConfirm] = useState<Iso | null>(null)
  const needle = query.trim().toLowerCase()
  const shown = (data?.isos ?? []).filter((iso) => !needle || iso.name.toLowerCase().includes(needle))
  const full = data ? data.used >= data.limit : false

  const deleteButton = (iso: Iso) =>
    iso.status !== 'importing' && !iso.mounted_on ? (
      <Button size="sm" icon={TrashIcon} onClick={() => setConfirm(iso)}>
        {t('Verwijderen')}
      </Button>
    ) : null

  const meta = (iso: Iso) => [
    { icon: CpuChipIcon, label: iso.firmware === 'uefi' ? 'UEFI' : 'BIOS' },
    ...(iso.server_name ? [{ icon: ServerStackIcon, label: iso.server_name }] : []),
    { icon: CalendarIcon, label: formatDate(iso.created_at) },
  ]

  return (
    <>
      <PageHeader
        title={t("ISO's")}
        description={
          data && (
            <span className="flex items-center gap-2">
              <PlayCircleIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {data.can_import ? t("{used} van {limit} ISO's gebruikt", { used: data.used, limit: data.limit }) : t("Je hebt minimaal één actieve VPS nodig voordat je ISO's kunt importeren.")}
            </span>
          )
        }
        actions={
          <Button size="sm" icon={PlusIcon} disabled={!data?.can_import || full} onClick={() => setImporting(true)}>
            {t('ISO importeren')}
          </Button>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t("ISO's zoeken...")} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t("Geen ISO's")} description={needle ? t("Er zijn geen ISO's gevonden.") : t('Importeer een ISO via een publieke https-URL om je VPS ervan te laten opstarten.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((iso) => (
            <li key={iso.id} className="flex items-center gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-[22px] dark:border-white/10 dark:bg-white/10">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-3">
                  <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{iso.name}</p>
                  <IsoStatus iso={iso} />
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-x-6 gap-y-1">
                  {meta(iso).map(({ icon: Icon, label }) => (
                    <span key={label} className="flex items-center gap-2 text-sm text-zinc-500 dark:text-white/70">
                      <Icon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                      {label}
                    </span>
                  ))}
                </div>
                {iso.status === 'failed' && iso.last_error && <p className="mt-2 text-sm text-red-500 dark:text-red-400">{iso.last_error}</p>}
              </div>
              {deleteButton(iso)}
            </li>
          ))}
        </RowCardList>
      ) : (
        <DataTable<Iso>
          rows={shown}
          rowKey={(iso) => iso.id}
          columns={[
            { header: t('Naam'), cell: (iso) => iso.name },
            { header: t('Status'), cell: (iso) => <IsoStatus iso={iso} /> },
            { header: t('Firmware'), cell: (iso) => (iso.firmware === 'uefi' ? 'UEFI' : 'BIOS') },
            { header: t('Server'), cell: (iso) => iso.server_name ?? '-' },
            { header: '', cell: deleteButton },
          ]}
        />
      )}
      {errorText(remove.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(remove.error)}</p>}
      <ImportIso open={importing} onClose={() => setImporting(false)} />
      <ConfirmDialog
        open={confirm !== null}
        title={t('ISO verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => confirm && remove.mutate(confirm.id, { onSettled: () => setConfirm(null) })}
      >
        {t('{name} is daarna weg. Je kunt hem later opnieuw importeren.', { name: confirm?.name ?? '' })}
      </ConfirmDialog>
    </>
  )
}
