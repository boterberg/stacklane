import { CircleStackIcon, CreditCardIcon, LinkIcon, LinkSlashIcon, PlusIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Modal } from '../../components/ui/Modal'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { formatMoney } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useVpsAccess } from '../../lib/vpsAccess'
import { intervalLabels, serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { useOrderVolume, useVolumeAction, useVolumeOptions, useVolumes, type Volume } from '../../lib/volumes'

const intervals = [{ value: 'monthly' }, { value: 'quarterly' }, { value: 'half_yearly' }, { value: 'yearly' }]

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function OrderVolume({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { data: options = [] } = useVolumeOptions(open)
  const order = useOrderVolume()
  const navigate = useNavigate()
  const [serverId, setServerId] = useState('')
  const [name, setName] = useState('')
  const [size, setSize] = useState('10')
  const [interval, setInterval] = useState('monthly')
  const option = options.find((item) => item.server_id === serverId) ?? options[0]
  const sizeGb = Math.round(Number(size)) || 0
  const perGb = option?.price_per_gb_cents ?? 0
  const errors = order.error instanceof ApiError ? order.error.fieldErrors : {}

  function close() {
    order.reset()
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!option) return
    order.mutate(
      { product_id: option.product_id, server_id: option.server_id, name: name.trim() || t('volume'), size_gb: sizeGb, billing_interval: interval },
      { onSuccess: ({ invoice_id }) => navigate(`/billing/invoices/${invoice_id}`) },
    )
  }

  return (
    <Modal
      open={open}
      onClose={close}
      title={t('Volume bestellen')}
      description={t('Extra opslag om aan een server te koppelen.')}
      footer={
        <>
          <p className="text-sm text-zinc-500 dark:text-white/70">
            {t('Totaal per maand')} <span className="font-medium text-zinc-800 dark:text-white">{formatMoney(perGb * sizeGb)}</span>
          </p>
          <Button variant="primary" type="submit" form="order-volume" disabled={!option || order.isPending}>
            {order.isPending ? t('Bezig...') : t('Bestellen')}
          </Button>
        </>
      }
    >
      {options.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Er zijn nog geen volumes te bestellen voor je servers.')}</p>
      ) : (
        <form id="order-volume" onSubmit={submit} className="space-y-5" noValidate>
          <Field label={t('Grootte')} description={t('Minimaal 10 GB, maximaal 1 TB.')} error={errors.size_gb}>
            {(id) => (
              <div className="flex">
                <div className="min-w-0 flex-1">
                  <Input id={id} icon={CircleStackIcon} inputMode="numeric" className="rounded-r-none" invalid={!!errors.size_gb} value={size} onChange={(event) => setSize(event.target.value)} />
                </div>
                <span className="inline-flex items-center rounded-r-lg border border-l-0 border-zinc-200 bg-zinc-50 px-3 text-sm text-zinc-500 dark:border-white/10 dark:bg-white/5 dark:text-white/70">
                  GB
                </span>
              </div>
            )}
          </Field>
          <Field label={t('Server')} error={errors.server_id}>
            {(id) => (
              <Select id={id} value={option?.server_id ?? ''} onChange={(event) => setServerId(event.target.value)}>
                {options.map((item) => (
                  <option key={item.server_id} value={item.server_id}>
                    {item.server_name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t('Naam')} error={errors.name}>
            {(id) => <Input id={id} placeholder={t('Bijvoorbeeld data')} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
          </Field>
          <Field label={t('Betaalinterval')} error={errors.billing_interval}>
            {(id) => (
              <Select id={id} value={interval} onChange={(event) => setInterval(event.target.value)}>
                {intervals.map((item) => (
                  <option key={item.value} value={item.value}>
                    {intervalLabels[item.value]} · {t('{price} per GB per maand', { price: formatMoney(perGb) })}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          {errors.product_id && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.product_id}</p>}
        </form>
      )}
    </Modal>
  )
}

function AttachVolume({ volume, onClose }: { volume: Volume | null; onClose: () => void }) {
  const { data: options = [] } = useVolumeOptions(volume !== null)
  const action = useVolumeAction()
  const [serverId, setServerId] = useState('')
  const chosen = serverId || options[0]?.server_id || ''

  function close() {
    action.reset()
    setServerId('')
    onClose()
  }

  return (
    <SlideOver
      open={volume !== null}
      onClose={close}
      title={t('Volume koppelen')}
      description={t('Kies de server waar {name} aan komt. Dat kan alleen een server op hetzelfde platform.', { name: volume?.name ?? '' })}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" disabled={!chosen || action.isPending} onClick={() => volume && action.mutate({ id: volume.id, action: 'attach', serverId: chosen }, { onSuccess: close })}>
            {action.isPending ? t('Bezig...') : t('Koppelen')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        <Field label={t('Server')}>
          {(id) => (
            <Select id={id} value={chosen} onChange={(event) => setServerId(event.target.value)}>
              {options.map((item) => (
                <option key={item.server_id} value={item.server_id}>
                  {item.server_name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        {errorText(action.error) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
      </div>
    </SlideOver>
  )
}

export function VolumesPage() {
  const { data, isPending } = useVolumes()
  const isOwner = useVpsAccess().data?.is_owner !== false
  const action = useVolumeAction()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [ordering, setOrdering] = useState(false)
  const [attaching, setAttaching] = useState<Volume | null>(null)
  const needle = query.trim().toLowerCase()
  const shown = (data?.volumes ?? []).filter((volume) => !needle || volume.name.toLowerCase().includes(needle) || (volume.server_name ?? '').toLowerCase().includes(needle))

  const status = (volume: Volume) => (volume.cancelled ? <Badge color="amber">{t('Opgezegd')}</Badge> : <Badge color={serviceStatusColors[volume.status]}>{serviceStatusLabels[volume.status]}</Badge>)

  const buttons = (volume: Volume) =>
    volume.status === 'active' ? (
      <div className="flex gap-2">
        {volume.server_id ? (
          <Button size="sm" icon={LinkSlashIcon} disabled={action.isPending} onClick={() => action.mutate({ id: volume.id, action: 'detach' })}>
            {t('Loskoppelen')}
          </Button>
        ) : (
          <Button size="sm" icon={LinkIcon} onClick={() => setAttaching(volume)}>
            {t('Koppelen')}
          </Button>
        )}
        {volume.subscription_code && (
          <ButtonLink size="sm" icon={CreditCardIcon} to={`/billing/subscriptions/${volume.subscription_code}`}>
            {t('Abonnement')}
          </ButtonLink>
        )}
      </div>
    ) : null

  const server = (volume: Volume) => volume.server_name ?? t('Niet gekoppeld')

  return (
    <>
      <PageHeader
        title={t('Volumes')}
        description={
          data && (
            <span className="flex items-center gap-2">
              <CircleStackIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('{used} van {limit} volumes gebruikt', { used: data.used, limit: data.limit })}
            </span>
          )
        }
        actions={
          isOwner && (
            <Button size="sm" icon={PlusIcon} onClick={() => setOrdering(true)}>
              {t('Volume bestellen')}
            </Button>
          )
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Volumes zoeken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen volumes')} description={needle ? t('Er zijn geen volumes gevonden.') : t('Bestel een volume om te beginnen.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((volume) => (
            <RowCard
              key={volume.id}
              to={volume.server_id ? `/vps/servers/${volume.server_id}` : '/vps/volumes'}
              title={volume.name}
              badge={status(volume)}
              meta={[
                { icon: CircleStackIcon, label: volume.size_gb ? t('{size} GB', { size: volume.size_gb }) : '-' },
                { icon: ServerStackIcon, label: server(volume) },
              ]}
              aside={buttons(volume)}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<Volume>
          rows={shown}
          rowKey={(volume) => volume.id}
          columns={[
            { header: t('Naam'), cell: (volume) => volume.name },
            { header: t('Status'), cell: status },
            { header: t('Grootte'), cell: (volume) => (volume.size_gb ? t('{size} GB', { size: volume.size_gb }) : '-') },
            {
              header: t('Server'),
              cell: (volume) =>
                volume.server_id ? (
                  <Link
                    to={`/vps/servers/${volume.server_id}`}
                    className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                  >
                    {volume.server_name}
                  </Link>
                ) : (
                  server(volume)
                ),
            },
            { header: '', cell: buttons },
          ]}
        />
      )}
      {errorText(action.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
      <OrderVolume open={ordering} onClose={() => setOrdering(false)} />
      <AttachVolume volume={attaching} onClose={() => setAttaching(null)} />
    </>
  )
}
