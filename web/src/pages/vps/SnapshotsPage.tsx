import { ArrowPathIcon, CalendarIcon, CameraIcon, PlusIcon, ServerStackIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link } from 'react-router'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useAllSnapshots, useSnapshotAction, useVpsServers, type ServerSnapshot } from '../../lib/vps'

function created(snapshot: ServerSnapshot) {
  return snapshot.created_at ? formatDate(new Date(snapshot.created_at * 1000).toISOString()) : '-'
}

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function CreateSnapshot({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { data: servers = [] } = useVpsServers()
  const active = servers.filter((server) => server.status === 'active')
  const [serverId, setServerId] = useState('')
  const [description, setDescription] = useState('')
  const create = useSnapshotAction()
  const chosen = serverId || active[0]?.id || ''

  function close() {
    create.reset()
    setDescription('')
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!chosen) return
    create.mutate({ serverId: chosen, action: 'create', description }, { onSuccess: close })
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={t('Snapshot maken')}
      description={t('Een momentopname van de schijf. Je kunt er later naar terug. Per server heb je er maximaal drie.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" type="submit" form="create-snapshot" disabled={!chosen || create.isPending}>
            {create.isPending ? t('Bezig...') : t('Snapshot maken')}
          </Button>
        </>
      }
    >
      <form id="create-snapshot" onSubmit={submit} className="space-y-6" noValidate>
        {active.length === 0 ? (
          <p className="text-sm text-zinc-500 dark:text-white/60">{t('Je hebt nog geen actieve server.')}</p>
        ) : (
          <Field label={t('Server')}>
            {(id) => (
              <Select id={id} value={chosen} onChange={(event) => setServerId(event.target.value)}>
                {active.map((server) => (
                  <option key={server.id} value={server.id}>
                    {server.name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        )}
        <Field label={t('Omschrijving')}>{(id) => <Input id={id} placeholder={t('Bijvoorbeeld voor de update')} value={description} onChange={(event) => setDescription(event.target.value)} />}</Field>
        {errorText(create.error) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errorText(create.error)}</p>}
      </form>
    </SlideOver>
  )
}

export function SnapshotsPage() {
  const { data, isPending } = useAllSnapshots()
  const action = useSnapshotAction()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [creating, setCreating] = useState(false)
  const [confirm, setConfirm] = useState<{ snapshot: ServerSnapshot; action: 'rollback' | 'delete' } | null>(null)
  const needle = query.trim().toLowerCase()
  const shown = (data?.snapshots ?? []).filter(
    (snapshot) => !needle || snapshot.description.toLowerCase().includes(needle) || snapshot.name.toLowerCase().includes(needle) || snapshot.server_name.toLowerCase().includes(needle),
  )

  const buttons = (snapshot: ServerSnapshot) => (
    <div className="flex gap-2">
      <Button size="sm" icon={ArrowPathIcon} onClick={() => setConfirm({ snapshot, action: 'rollback' })}>
        {t('Terugzetten')}
      </Button>
      <Button size="sm" icon={TrashIcon} onClick={() => setConfirm({ snapshot, action: 'delete' })}>
        {t('Verwijderen')}
      </Button>
    </div>
  )

  return (
    <>
      <PageHeader
        title={t('Snapshots')}
        description={
          data && (
            <span className="flex items-center gap-2">
              <CameraIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('{used} van {limit} snapshots gebruikt', { used: data.used, limit: data.limit })}
            </span>
          )
        }
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            {t('Snapshot maken')}
          </Button>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Snapshots zoeken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen snapshots')} description={needle ? t('Er zijn geen snapshots gevonden.') : t('Maak een snapshot van een van je VPSen om te beginnen.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((snapshot) => (
            <RowCard
              key={`${snapshot.server_id}-${snapshot.name}`}
              to={`/vps/servers/${snapshot.server_id}`}
              title={snapshot.description || snapshot.name}
              meta={[
                { icon: ServerStackIcon, label: snapshot.server_name },
                { icon: CalendarIcon, label: created(snapshot) },
              ]}
              aside={buttons(snapshot)}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<ServerSnapshot>
          rows={shown}
          rowKey={(snapshot) => `${snapshot.server_id}-${snapshot.name}`}
          columns={[
            { header: t('Snapshot'), cell: (snapshot) => snapshot.description || snapshot.name },
            {
              header: t('Server'),
              cell: (snapshot) => (
                <Link
                  to={`/vps/servers/${snapshot.server_id}`}
                  className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                >
                  {snapshot.server_name}
                </Link>
              ),
            },
            { header: t('Gemaakt'), cell: created },
            { header: '', cell: buttons },
          ]}
        />
      )}
      {errorText(action.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
      <CreateSnapshot open={creating} onClose={() => setCreating(false)} />
      <ConfirmDialog
        open={confirm !== null}
        title={confirm?.action === 'rollback' ? t('Snapshot terugzetten?') : t('Snapshot verwijderen?')}
        confirmLabel={confirm?.action === 'rollback' ? t('Terugzetten') : t('Verwijderen')}
        danger
        pending={action.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => {
          if (!confirm) return
          action.mutate({ serverId: confirm.snapshot.server_id, action: confirm.action, name: confirm.snapshot.name }, { onSettled: () => setConfirm(null) })
        }}
      >
        {confirm?.action === 'rollback'
          ? t('{server} gaat terug naar deze snapshot. Wat er daarna veranderd is, ben je kwijt.', { server: confirm.snapshot.server_name })
          : t('Deze snapshot is daarna weg.')}
      </ConfirmDialog>
    </>
  )
}
