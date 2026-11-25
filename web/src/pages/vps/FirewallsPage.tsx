import { CalendarIcon, ListBulletIcon, PlusIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { useFirewalls, useSaveFirewall, type AccountFirewall } from '../../lib/firewall'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'

function CreateFirewall({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [name, setName] = useState('')
  const [autoAssign, setAutoAssign] = useState(false)
  const save = useSaveFirewall()
  const navigate = useNavigate()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}

  function close() {
    save.reset()
    setName('')
    setAutoAssign(false)
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    save.mutate({ input: { name, auto_assign: autoAssign, rules: [] } }, { onSuccess: ({ id }) => navigate(`/vps/firewalls/${id}`) })
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={t('Firewall maken')}
      description={t('Een firewall is een set regels die je aan meerdere servers koppelt. Pas je de regels aan, dan krijgen al die servers ze meteen.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" type="submit" form="create-firewall" disabled={save.isPending}>
            {save.isPending ? t('Bezig...') : t('Firewall maken')}
          </Button>
        </>
      }
    >
      <form id="create-firewall" onSubmit={submit} className="space-y-6" noValidate>
        <Field label={t('Naam')} error={errors.name}>
          {(id) => <Input id={id} placeholder={t('Bijvoorbeeld Webservers')} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>
        <div>
          <Checkbox label={t('Automatisch toewijzen')} checked={autoAssign} onChange={(event) => setAutoAssign(event.target.checked)} />
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{t('Nieuwe servers krijgen deze firewall vanzelf.')}</p>
        </div>
      </form>
    </SlideOver>
  )
}

function rulesLabel(count: number) {
  return count === 1 ? t('1 firewallregel') : t('{count} firewallregels', { count })
}

function serversLabel(count: number) {
  return count === 1 ? t('1 server') : t('{count} servers', { count })
}

export function FirewallsPage() {
  const { data: firewalls = [], isPending } = useFirewalls()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [creating, setCreating] = useState(false)
  const needle = query.trim().toLowerCase()
  const shown = firewalls.filter((firewall) => !needle || firewall.name.toLowerCase().includes(needle))
  const automatic = <Badge color="blue">{t('Automatisch toewijzen')}</Badge>

  return (
    <>
      <PageHeader
        title={t('Firewalls')}
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            {t('Firewall maken')}
          </Button>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Firewalls zoeken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen firewalls')} description={needle ? t('Er zijn geen firewalls gevonden.') : t('Maak een firewall en koppel die aan je servers.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((firewall) => (
            <RowCard
              key={firewall.id}
              to={`/vps/firewalls/${firewall.id}`}
              title={firewall.name}
              badge={firewall.auto_assign ? automatic : undefined}
              meta={[
                { icon: ListBulletIcon, label: rulesLabel(firewall.rules) },
                { icon: ServerStackIcon, label: serversLabel(firewall.servers) },
                { icon: CalendarIcon, label: formatDate(firewall.created_at) },
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<AccountFirewall>
          rows={shown}
          rowKey={(firewall) => firewall.id}
          columns={[
            {
              header: t('Naam'),
              cell: (firewall) => (
                <Link
                  to={`/vps/firewalls/${firewall.id}`}
                  className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                >
                  {firewall.name}
                </Link>
              ),
            },
            { header: t('Regels'), cell: (firewall) => firewall.rules },
            { header: t('Servers'), cell: (firewall) => firewall.servers },
            { header: '', cell: (firewall) => (firewall.auto_assign ? automatic : null) },
            { header: t('Gemaakt'), cell: (firewall) => formatDate(firewall.created_at) },
          ]}
        />
      )}
      <CreateFirewall open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
