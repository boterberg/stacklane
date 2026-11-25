import { CalendarIcon, GlobeAltIcon, PlusIcon, ServerStackIcon, Square3Stack3DIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { Modal } from '../../components/ui/Modal'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useCreateNetwork, useNetworks, type Network, type NetworkOverview } from '../../lib/networks'
import { useCan } from '../../lib/vpsAccess'

const prefixes = ['16', '20', '24', '26', '28']

function CreateNetwork({ overview, open, onClose }: { overview: NetworkOverview; open: boolean; onClose: () => void }) {
  const [base, suggestedPrefix] = overview.suggested_cidr.split('/')
  const [name, setName] = useState(overview.suggested_name)
  const [address, setAddress] = useState(base ?? '')
  const [prefix, setPrefix] = useState(suggestedPrefix ?? '24')
  const [platform, setPlatform] = useState('')
  const create = useCreateNetwork()
  const navigate = useNavigate()
  const chosen = overview.platforms.find((item) => item.integration_id === platform) ?? overview.platforms[0]
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function close() {
    create.reset()
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!chosen) return
    create.mutate(
      { name: name.trim(), cidr: `${address.trim()}/${prefix}`, integration_id: chosen.integration_id },
      { onSuccess: ({ id }) => navigate(`/vps/networks/${id}`) },
    )
  }

  return (
    <Modal
      open={open}
      onClose={close}
      title={t('Creëer een netwerk')}
      description={t('Met een netwerk kunnen je servers onderling communiceren.')}
      footer={
        <>
          <span />
          <Button variant="primary" type="submit" form="create-network" disabled={!chosen || create.isPending}>
            {create.isPending ? t('Bezig...') : t('Maken')}
          </Button>
        </>
      }
    >
      {overview.platforms.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Er is nog geen platform waarop je netwerken kunt maken.')}</p>
      ) : (
        <form id="create-network" onSubmit={submit} className="space-y-5" noValidate>
          <Field label={t('Naam')} error={errors.name}>
            {(id) => <Input id={id} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
          </Field>
          <Field label={t('IP-bereik')} error={errors.cidr}>
            {(id) => (
              <div className="flex">
                <div className="min-w-0 flex-1">
                  <Input id={id} className="rounded-r-none" invalid={!!errors.cidr} value={address} onChange={(event) => setAddress(event.target.value)} />
                </div>
                <div className="w-24 shrink-0">
                  <Select aria-label={t('Netmasker')} className="rounded-l-none border-l-0" value={prefix} onChange={(event) => setPrefix(event.target.value)}>
                    {prefixes.map((value) => (
                      <option key={value} value={value}>
                        /{value}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>
            )}
          </Field>
          {overview.platforms.length > 1 && (
            <Field label={t('Locatie')} error={errors.integration_id}>
              {(id) => (
                <Select id={id} value={chosen?.integration_id ?? ''} onChange={(event) => setPlatform(event.target.value)}>
                  {overview.platforms.map((item) => (
                    <option key={item.integration_id} value={item.integration_id}>
                      {item.name}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          )}
          {overview.platforms.length === 1 && errors.integration_id && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.integration_id}</p>}
        </form>
      )}
    </Modal>
  )
}

function serversLabel(count: number) {
  return count === 1 ? t('1 server') : t('{count} servers', { count })
}

export function NetworksPage() {
  const { data, isPending } = useNetworks()
  const canCreate = useCan()('networks.create')
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [creating, setCreating] = useState(false)
  const networks = data?.networks ?? []
  const needle = query.trim().toLowerCase()
  const shown = networks.filter((network) => !needle || network.name.toLowerCase().includes(needle) || network.cidr.includes(needle))
  const full = data ? networks.length >= data.limit : true

  return (
    <>
      <PageHeader
        title={t('Netwerken')}
        actions={
          canCreate && (
            <Button size="sm" icon={PlusIcon} disabled={full} onClick={() => setCreating(true)}>
              {t('Netwerk maken')}
            </Button>
          )
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Zoek netwerken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen netwerken')} description={needle ? t('Er zijn geen netwerken gevonden.') : t('Begin met het maken van een netwerk.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((network) => (
            <RowCard
              key={network.id}
              to={`/vps/networks/${network.id}`}
              title={network.name}
              meta={[
                { icon: Square3Stack3DIcon, label: t('1 subnet') },
                { icon: ServerStackIcon, label: serversLabel(network.servers) },
                { icon: GlobeAltIcon, label: network.cidr },
                { icon: CalendarIcon, label: formatDate(network.created_at) },
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<Network>
          rows={shown}
          rowKey={(network) => network.id}
          columns={[
            {
              header: t('Naam'),
              cell: (network) => (
                <Link
                  to={`/vps/networks/${network.id}`}
                  className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                >
                  {network.name}
                </Link>
              ),
            },
            { header: t('IP-bereik'), cell: (network) => network.cidr },
            { header: t('Servers'), cell: (network) => network.servers },
            { header: t('Locatie'), cell: (network) => network.platform },
            { header: t('Gemaakt'), cell: (network) => formatDate(network.created_at) },
          ]}
        />
      )}
      {data && creating && <CreateNetwork overview={data} open={creating} onClose={() => setCreating(false)} />}
    </>
  )
}
