import { ArrowsRightLeftIcon, CommandLineIcon, CreditCardIcon, LinkIcon, LinkSlashIcon, MapPinIcon, PlusIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { Modal } from '../../components/ui/Modal'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { ApiError } from '../../lib/api'
import { useFloatingIpAction, useFloatingIpOptions, useFloatingIps, useOrderFloatingIps, type FloatingIp } from '../../lib/floatingIps'
import { formatMoney } from '../../lib/format'
import { t } from '../../lib/i18n'
import { intervalLabels, serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { useVpsAccess } from '../../lib/vpsAccess'

const intervals = ['monthly', 'quarterly', 'half_yearly', 'yearly']

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function OrderFloatingIps({ open, left, onClose }: { open: boolean; left: number; onClose: () => void }) {
  const { data } = useFloatingIpOptions(open)
  const products = data?.products ?? []
  const order = useOrderFloatingIps()
  const navigate = useNavigate()
  const [productId, setProductId] = useState('')
  const [quantity, setQuantity] = useState('1')
  const [interval, setInterval] = useState('monthly')
  const product = products.find((item) => item.product_id === productId) ?? products[0]
  const count = Math.round(Number(quantity)) || 0
  const price = product?.monthly_price_cents ?? 0
  const errors = order.error instanceof ApiError ? order.error.fieldErrors : {}

  function close() {
    order.reset()
    onClose()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!product) return
    order.mutate(
      { product_id: product.product_id, quantity: count, billing_interval: interval },
      { onSuccess: ({ invoice_id }) => navigate(`/billing/invoices/${invoice_id}`) },
    )
  }

  return (
    <Modal
      open={open}
      onClose={close}
      title={t('Floating IP bestellen')}
      description={t('Een vast IP-adres dat je op elk moment naar een andere server verplaatst.')}
      footer={
        <>
          <p className="text-sm text-zinc-500 dark:text-white/70">
            {t('Totaal per maand')} <span className="font-medium text-zinc-800 dark:text-white">{formatMoney(price * Math.max(count, 0))}</span>
          </p>
          <Button variant="primary" type="submit" form="order-floating-ips" disabled={!product || left === 0 || order.isPending}>
            {order.isPending ? t('Bezig...') : t('Bestellen')}
          </Button>
        </>
      }
    >
      {products.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Er zijn nog geen floating IPs te bestellen.')}</p>
      ) : (
        <form id="order-floating-ips" onSubmit={submit} className="space-y-5" noValidate>
          <Field
            label={t("Aantal IP's")}
            description={t('{price} per IP per maand. Je kunt er nog {left} bestellen.', { price: formatMoney(price), left })}
            error={errors.quantity}
          >
            {(id) => <Input id={id} icon={MapPinIcon} inputMode="numeric" invalid={!!errors.quantity} value={quantity} onChange={(event) => setQuantity(event.target.value)} />}
          </Field>
          {products.length > 1 && (
            <Field label={t('Locatie')} error={errors.product_id}>
              {(id) => (
                <Select id={id} value={product?.product_id ?? ''} onChange={(event) => setProductId(event.target.value)}>
                  {products.map((item) => (
                    <option key={item.product_id} value={item.product_id}>
                      {item.name}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          )}
          <Field label={t('Betaalinterval')} error={errors.billing_interval}>
            {(id) => (
              <Select id={id} value={interval} onChange={(event) => setInterval(event.target.value)}>
                {intervals.map((value) => (
                  <option key={value} value={value}>
                    {intervalLabels[value]}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          {products.length === 1 && errors.product_id && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.product_id}</p>}
        </form>
      )}
    </Modal>
  )
}

function AttachFloatingIp({ floating, onClose }: { floating: FloatingIp | null; onClose: () => void }) {
  const { data } = useFloatingIpOptions(floating !== null)
  const servers = (data?.servers ?? []).filter((server) => server.integration_id === floating?.integration_id && server.server_id !== floating?.server_id)
  const action = useFloatingIpAction()
  const [serverId, setServerId] = useState('')
  const chosen = servers.some((server) => server.server_id === serverId) ? serverId : (servers[0]?.server_id ?? '')
  const moving = Boolean(floating?.server_id)

  function close() {
    action.reset()
    setServerId('')
    onClose()
  }

  return (
    <SlideOver
      open={floating !== null}
      onClose={close}
      title={moving ? t('Floating IP verplaatsen') : t('Floating IP koppelen')}
      description={
        moving
          ? t('{address} gaat van {server} naar de server die je kiest.', { address: floating?.address ?? '', server: floating?.server_name ?? '' })
          : t('Kies de server waar {address} naartoe gaat. Dat kan alleen een server op hetzelfde platform.', { address: floating?.address ?? '' })
      }
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button
            variant="primary"
            disabled={!chosen || action.isPending}
            onClick={() => floating && action.mutate({ id: floating.id, action: 'attach', serverId: chosen }, { onSuccess: close })}
          >
            {action.isPending ? t('Bezig...') : moving ? t('Verplaatsen') : t('Koppelen')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        {servers.length === 0 ? (
          <p className="text-sm text-zinc-500 dark:text-white/60">{t('Je hebt geen andere actieve server op dit platform.')}</p>
        ) : (
          <Field label={t('Server')}>
            {(id) => (
              <Select id={id} value={chosen} onChange={(event) => setServerId(event.target.value)}>
                {servers.map((server) => (
                  <option key={server.server_id} value={server.server_id}>
                    {server.server_name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        )}
        {errorText(action.error) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
      </div>
    </SlideOver>
  )
}

function GuestSetup({ floating, onClose }: { floating: FloatingIp | null; onClose: () => void }) {
  const address = floating?.address ?? ''
  const commands = `ip address add ${address}/32 dev eth0\narping -U -c 3 -I eth0 ${address}`
  return (
    <SlideOver
      open={floating !== null}
      onClose={onClose}
      title={t('Floating IP instellen')}
      description={t('Op dit platform stel je het adres zelf in op je server. Doe dit na elke keer koppelen of verplaatsen.')}
    >
      <div className="space-y-5 text-sm text-zinc-600 dark:text-white/70">
        <p>{t('Voeg het adres toe aan de netwerkkaart en laat het netwerk weten waar het nu zit:')}</p>
        <pre className="overflow-x-auto rounded-lg border border-zinc-200 bg-zinc-50 p-4 font-mono text-xs text-zinc-800 dark:border-white/10 dark:bg-white/5 dark:text-white">{commands}</pre>
        <p>{t('Heet je netwerkkaart anders dan eth0, kijk dan met ip link welke naam hij heeft. Zet het adres ook in de netwerkinstellingen van je server, zoals netplan, zodat het na een herstart blijft.')}</p>
      </div>
    </SlideOver>
  )
}

export function FloatingIpsPage() {
  const { data, isPending } = useFloatingIps()
  const access = useVpsAccess().data
  const isOwner = access?.is_owner !== false
  const action = useFloatingIpAction()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const [ordering, setOrdering] = useState(false)
  const [attaching, setAttaching] = useState<FloatingIp | null>(null)
  const [explaining, setExplaining] = useState<FloatingIp | null>(null)
  const needle = query.trim().toLowerCase()
  const shown = (data?.floating_ips ?? []).filter(
    (floating) => !needle || (floating.address ?? '').includes(needle) || (floating.server_name ?? '').toLowerCase().includes(needle),
  )
  const left = data ? Math.max(data.limit - data.used, 0) : 0

  const title = (floating: FloatingIp) => floating.address ?? t('Nieuw IP-adres')
  const status = (floating: FloatingIp) =>
    floating.cancelled ? <Badge color="amber">{t('Opgezegd')}</Badge> : <Badge color={serviceStatusColors[floating.status]}>{serviceStatusLabels[floating.status]}</Badge>
  const server = (floating: FloatingIp) => floating.server_name ?? t('Niet gekoppeld')

  const buttons = (floating: FloatingIp) =>
    floating.status === 'active' ? (
      <div className="flex flex-wrap gap-2">
        {floating.server_id ? (
          <>
            <Button size="sm" icon={ArrowsRightLeftIcon} onClick={() => setAttaching(floating)}>
              {t('Verplaatsen')}
            </Button>
            <Button size="sm" icon={LinkSlashIcon} disabled={action.isPending} onClick={() => action.mutate({ id: floating.id, action: 'detach' })}>
              {t('Ontkoppelen')}
            </Button>
            {floating.guest_setup && (
              <Button size="sm" icon={CommandLineIcon} onClick={() => setExplaining(floating)}>
                {t('Instellen')}
              </Button>
            )}
          </>
        ) : (
          <Button size="sm" icon={LinkIcon} onClick={() => setAttaching(floating)}>
            {t('Koppelen')}
          </Button>
        )}
        {isOwner && floating.subscription_code && (
          <ButtonLink size="sm" icon={CreditCardIcon} to={`/billing/subscriptions/${floating.subscription_code}`}>
            {t('Abonnement')}
          </ButtonLink>
        )}
      </div>
    ) : null

  return (
    <>
      <PageHeader
        title={t('Floating IPs')}
        description={
          data && (
            <span className="flex items-center gap-2">
              <MapPinIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('{used} van {limit} floating IPs gebruikt', { used: data.used, limit: data.limit })}
            </span>
          )
        }
        actions={
          isOwner && (
            <Button size="sm" icon={PlusIcon} disabled={!data || left === 0} onClick={() => setOrdering(true)}>
              {t('Floating IP bestellen')}
            </Button>
          )
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Floating IPs zoeken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState
          title={t('Geen floating IPs')}
          description={needle ? t('Er zijn geen floating IPs gevonden.') : isOwner ? t('Bestel een floating IP om te beginnen.') : t('Er zijn nog geen floating IPs.')}
        />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((floating) => (
            <RowCard
              key={floating.id}
              to={floating.server_id ? `/vps/servers/${floating.server_id}` : '/vps/floating-ips'}
              title={title(floating)}
              badge={status(floating)}
              meta={[{ icon: ServerStackIcon, label: server(floating) }]}
              aside={buttons(floating)}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<FloatingIp>
          rows={shown}
          rowKey={(floating) => floating.id}
          columns={[
            { header: t('IP-adres'), cell: title },
            { header: t('Status'), cell: status },
            {
              header: t('Server'),
              cell: (floating) =>
                floating.server_id ? (
                  <Link
                    to={`/vps/servers/${floating.server_id}`}
                    className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                  >
                    {floating.server_name}
                  </Link>
                ) : (
                  server(floating)
                ),
            },
            { header: '', cell: buttons },
          ]}
        />
      )}
      {errorText(action.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
      <OrderFloatingIps open={ordering} left={left} onClose={() => setOrdering(false)} />
      <AttachFloatingIp floating={attaching} onClose={() => setAttaching(null)} />
      <GuestSetup floating={explaining} onClose={() => setExplaining(null)} />
    </>
  )
}
