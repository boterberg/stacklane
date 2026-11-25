import { ArrowPathIcon, ArrowTopRightOnSquareIcon, CameraIcon, ExclamationTriangleIcon, EyeIcon, GlobeAltIcon, PlayIcon, PowerIcon, ServerIcon, StopIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input } from '../../components/ui/Field'
import { Tabs } from '../../components/ui/Tabs'
import { ApiError } from '../../lib/api'
import { formatBytes, formatDate, formatDuration } from '../../lib/format'
import { t } from '../../lib/i18n'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import {
  specsLine,
  useAddFirewallRule,
  useCreateSnapshot,
  useDeleteFirewallRule,
  useDeleteSnapshot,
  useFirewall,
  usePower,
  useRollbackSnapshot,
  usePanelLogin,
  useRootPassword,
  useSnapshots,
  useVpsServer,
  type Features,
  type LiveStatus,
  type Snapshot,
  type VpsServer,
} from '../../lib/vps'
import { isManaged } from '../../lib/firewall'
import { RuleForm, RuleList } from './FirewallRules'
import { LinkedCard, ServerFloatingIps, ServerNetworks, ServerVolumes } from './ServerAttached'
import { ServerGraphs } from './ServerGraphs'
import { ServerIso } from './ServerIso'
import { ServerRebuild } from './ServerRebuild'
import { useCan, useVpsAccess } from '../../lib/vpsAccess'

type Tab = 'overview' | 'graphs' | 'networks' | 'firewall' | 'floating' | 'volumes' | 'snapshots' | 'rebuild' | 'iso'

function errorText(error: Error | null) {
  return error instanceof ApiError ? (Object.values(error.fieldErrors)[0] ?? error.message) : error?.message
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="py-3 first:pt-0 last:pb-0">
      <dt className="text-sm font-medium text-zinc-800 dark:text-white">{label}</dt>
      <dd className="mt-1 text-sm text-zinc-500 dark:text-white/70">{children}</dd>
    </div>
  )
}

function Overview({
  server,
  live,
  features,
  linked,
  onOpen,
}: {
  server: VpsServer
  live: LiveStatus | null
  features: Features | null
  linked: { volumes: boolean; floating: boolean; networks: boolean }
  onOpen: (tab: Tab) => void
}) {
  const password = useRootPassword(server.id)
  const isOwner = useVpsAccess().data?.is_owner !== false
  return (
    <div className="grid max-w-5xl gap-6 lg:grid-cols-2">
      <Card title={t('Server')}>
        <dl className="divide-y divide-zinc-200 dark:divide-white/10">
          <Row label={t('IP-adres')}>{server.details.ipv4 ?? '-'}</Row>
          <Row label={t('Hostname')}>{server.details.hostname ?? '-'}</Row>
          <Row label={t('Besturingssysteem')}>{server.details.image ?? '-'}</Row>
          <Row label={t('Pakket')}>
            {server.product_name} · {specsLine(server.details)}
          </Row>
          <Row label={t('Inloggen')}>
            <span className="block">ssh root@{server.details.ipv4}</span>
            {features?.panel ? (
              <span className="mt-1 block">{t('Het rootwachtwoord en de console vind je in het panel van VirtFusion.')}</span>
            ) : !isOwner ? (
              <span className="mt-1 block">{t('Alleen de eigenaar van de server kan het rootwachtwoord zien.')}</span>
            ) : password.data ? (
              <code className="mt-2 block font-mono text-zinc-800 dark:text-white">{password.data.password ?? t('Niet beschikbaar')}</code>
            ) : (
              <Button size="sm" icon={EyeIcon} className="mt-2" disabled={password.isPending} onClick={() => password.mutate()}>
                {t('Rootwachtwoord tonen')}
              </Button>
            )}
          </Row>
        </dl>
      </Card>
      <Card title={t('Gebruik')}>
        {live ? (
          <dl className="divide-y divide-zinc-200 dark:divide-white/10">
            <Row label={t('Status')}>{live.state === 'running' ? t('Draait') : t('Uit')}</Row>
            <Row label={t('Processor')}>
              {live.cpu === null ? t('{cores} vCPU', { cores: live.cpus }) : t('{percent}% van {cores} vCPU', { percent: Math.round(live.cpu * 100), cores: live.cpus })}
            </Row>
            <Row label={t('Geheugen')}>{live.memory === null ? formatBytes(live.max_memory) : t('{used} van {total}', { used: formatBytes(live.memory), total: formatBytes(live.max_memory) })}</Row>
            {live.uptime !== null && <Row label={t('Uptime')}>{live.uptime > 0 ? formatDuration(live.uptime * 1000) : '-'}</Row>}
          </dl>
        ) : (
          <p className="text-sm text-zinc-500 dark:text-white/70">{t('De server geeft nu geen gegevens door.')}</p>
        )}
      </Card>
      <LinkedCard serverId={server.id} show={linked} onOpen={onOpen} />
    </div>
  )
}

function Snapshots({ server }: { server: VpsServer }) {
  const { data: snapshots, isPending } = useSnapshots(server.id, true)
  const create = useCreateSnapshot(server.id)
  const rollback = useRollbackSnapshot(server.id)
  const remove = useDeleteSnapshot(server.id)
  const [description, setDescription] = useState('')
  const [confirm, setConfirm] = useState<{ snapshot: Snapshot; action: 'rollback' | 'delete' } | null>(null)
  const error = errorText(create.error ?? rollback.error ?? remove.error)

  return (
    <div className="max-w-3xl space-y-6">
      <Card title={t('Snapshot maken')} description={t('Een momentopname van de schijf. Je kunt er later naar terug. Je hebt er maximaal drie.')}>
        <form
          className="flex flex-wrap items-end gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            create.mutate(description, { onSuccess: () => setDescription('') })
          }}
        >
          <div className="min-w-48 flex-1">
            <Field label={t('Omschrijving')}>
              {(id) => <Input id={id} placeholder={t('Bijvoorbeeld voor de update')} value={description} onChange={(event) => setDescription(event.target.value)} />}
            </Field>
          </div>
          <Button type="submit" variant="primary" icon={CameraIcon} disabled={create.isPending}>
            {create.isPending ? t('Bezig...') : t('Snapshot maken')}
          </Button>
        </form>
        {error && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
      </Card>
      {!isPending && (snapshots ?? []).length === 0 ? (
        <EmptyState title={t('Geen snapshots')} description={t('Je hebt nog geen snapshots van deze server.')} />
      ) : (
        <ul className="space-y-3">
          {(snapshots ?? []).map((snapshot) => (
            <li key={snapshot.name} className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-4 dark:border-white/10 dark:bg-white/10">
              <div>
                <p className="text-sm font-medium text-zinc-800 dark:text-white">{snapshot.description || snapshot.name}</p>
                <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{snapshot.created_at ? formatDate(new Date(snapshot.created_at * 1000).toISOString()) : snapshot.name}</p>
              </div>
              <div className="flex gap-2">
                <Button size="sm" icon={ArrowPathIcon} onClick={() => setConfirm({ snapshot, action: 'rollback' })}>
                  {t('Terugzetten')}
                </Button>
                <Button size="sm" icon={TrashIcon} onClick={() => setConfirm({ snapshot, action: 'delete' })}>
                  {t('Verwijderen')}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={confirm !== null}
        title={confirm?.action === 'rollback' ? t('Snapshot terugzetten?') : t('Snapshot verwijderen?')}
        confirmLabel={confirm?.action === 'rollback' ? t('Terugzetten') : t('Verwijderen')}
        danger
        pending={rollback.isPending || remove.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => {
          if (!confirm) return
          const done = { onSettled: () => setConfirm(null) }
          if (confirm.action === 'rollback') rollback.mutate(confirm.snapshot.name, done)
          else remove.mutate(confirm.snapshot.name, done)
        }}
      >
        {confirm?.action === 'rollback' ? t('Alles wat na deze snapshot op de server is veranderd, gaat verloren.') : t('De snapshot is daarna weg.')}
      </ConfirmDialog>
    </div>
  )
}

function Firewall({ server }: { server: VpsServer }) {
  const { data: rules, isPending } = useFirewall(server.id, true)
  const add = useAddFirewallRule(server.id)
  const remove = useDeleteFirewallRule(server.id)
  const errors = add.error instanceof ApiError ? add.error.fieldErrors : {}

  return (
    <div className="max-w-4xl space-y-6">
      <Card title={t('Regel toevoegen')} description={t('Zonder regels is al het verkeer toegestaan. Regels gelden van boven naar beneden.')}>
        <RuleForm pending={add.isPending} errors={errors} onAdd={(rule, done) => add.mutate(rule, { onSuccess: done })} />
        {errors.server && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{errors.server}</p>}
      </Card>
      {!isPending && (rules ?? []).length === 0 ? (
        <EmptyState title={t('Geen regels')} description={t('Al het verkeer naar deze server is toegestaan.')} />
      ) : (
        <RuleList rules={rules ?? []} pending={remove.isPending} canDelete={(rule) => !isManaged(rule)} onDelete={(rule) => remove.mutate(rule.id)} />
      )}
    </div>
  )
}

function PanelButton({ id }: { id: string }) {
  const login = usePanelLogin(id)
  function open() {
    const tab = window.open('about:blank', '_blank')
    login.mutate(undefined, {
      onSuccess: ({ url }) => {
        if (tab) tab.location.href = url
        else window.location.assign(url)
      },
      onError: () => tab?.close(),
    })
  }
  return (
    <Button size="sm" icon={ArrowTopRightOnSquareIcon} disabled={login.isPending} onClick={open}>
      {t('Open in VirtFusion')}
    </Button>
  )
}

function PowerButtons({ server, live }: { server: VpsServer; live: LiveStatus | null }) {
  const power = usePower(server.id)
  const can = useCan()
  const [stopping, setStopping] = useState(false)
  if (server.status !== 'active' || !live) return null
  const running = live.state === 'running'
  return (
    <div className="flex flex-wrap gap-2">
      {running ? (
        <>
          {can('servers.start') && can('servers.stop') && (
            <Button size="sm" icon={ArrowPathIcon} disabled={power.isPending} onClick={() => power.mutate('reboot')}>
              {t('Herstarten')}
            </Button>
          )}
          {can('servers.stop') && (
            <>
              <Button size="sm" icon={PowerIcon} disabled={power.isPending} onClick={() => power.mutate('shutdown')}>
                {t('Afsluiten')}
              </Button>
              <Button size="sm" icon={StopIcon} disabled={power.isPending} onClick={() => setStopping(true)}>
                {t('Uitzetten')}
              </Button>
            </>
          )}
        </>
      ) : (
        can('servers.start') && (
          <Button size="sm" variant="primary" icon={PlayIcon} disabled={power.isPending} onClick={() => power.mutate('start')}>
            {t('Starten')}
          </Button>
        )
      )}
      <ConfirmDialog
        open={stopping}
        title={t('Server hard uitzetten?')}
        confirmLabel={t('Uitzetten')}
        danger
        pending={power.isPending}
        onClose={() => setStopping(false)}
        onConfirm={() => power.mutate('stop', { onSettled: () => setStopping(false) })}
      >
        {t('Alsof je de stekker eruit trekt. Gebruik liever Afsluiten, tenzij de server niet meer reageert.')}
      </ConfirmDialog>
    </div>
  )
}

export function VpsServerPage() {
  const { id = '' } = useParams()
  const { data: server, isPending, error } = useVpsServer(id)
  const [tab, setTab] = useState<Tab>('overview')
  const can = useCan()
  const { data: access } = useVpsAccess()

  if (isPending) return null
  if (error || !server) {
    return (
      <EmptyState
        title={t('Server niet gevonden')}
        description={t('Deze server bestaat niet of is van iemand anders.')}
        action={
          <ButtonLink to="/vps/servers" size="sm">
            {t('Naar servers')}
          </ButtonLink>
        }
      />
    )
  }

  const ready = server.status === 'active'
  const linked = {
    volumes: server.features?.volumes === true && can('volumes.view'),
    floating: server.features?.floating_ips === true && can('floating_ips.view'),
    networks: server.features?.networks === true && can('networks.view'),
  }
  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{server.name}</h1>
            {server.details.rebuilding ? (
              <Badge color="blue">{t('Wordt herbouwd')}</Badge>
            ) : (
              <Badge color={serviceStatusColors[server.status]}>{serviceStatusLabels[server.status]}</Badge>
            )}
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            {server.details.ipv4 && (
              <span className="flex items-center gap-2">
                <GlobeAltIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {server.details.ipv4}
              </span>
            )}
            {server.details.image && (
              <span className="flex items-center gap-2">
                <ServerIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {server.details.image}
              </span>
            )}
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          {server.features?.panel && access?.is_owner !== false && <PanelButton id={server.id} />}
          <PowerButtons server={server} live={server.live} />
        </div>
      </div>

      {!ready ? (
        <EmptyState
          title={server.status === 'failed' ? t('Er ging iets mis') : t('Je server wordt klaargezet')}
          description={
            server.status === 'failed'
              ? t('We konden je server niet aanmaken. Ons team is op de hoogte.')
              : server.status === 'pending'
                ? t('Zodra de factuur betaald is, maken we je server aan.')
                : t('Dit duurt meestal een paar minuten.')
          }
        />
      ) : (
        <>
          {server.last_error && !server.details.rebuilding && (
            <div className="mb-6 flex gap-3 rounded-xl border border-red-200 bg-red-50 px-6 py-4 text-sm text-red-800 dark:border-red-400/20 dark:bg-red-400/10 dark:text-red-300">
              <ExclamationTriangleIcon className="size-5 shrink-0" aria-hidden />
              <p>{server.last_error}</p>
            </div>
          )}
          <Tabs
            tabs={[
              { id: 'overview' as const, label: 'Overzicht' },
              ...(server.features?.graphs ? [{ id: 'graphs' as const, label: 'Grafieken' }] : []),
              ...(linked.networks ? [{ id: 'networks' as const, label: 'Netwerken' }] : []),
              ...(server.features?.firewall === false || !can('firewalls.view') ? [] : [{ id: 'firewall' as const, label: 'Firewalls' }]),
              ...(linked.floating ? [{ id: 'floating' as const, label: 'Floating IPs' }] : []),
              ...(linked.volumes ? [{ id: 'volumes' as const, label: 'Volumes' }] : []),
              ...(server.features?.snapshots === false || !can('snapshots.view') ? [] : [{ id: 'snapshots' as const, label: 'Snapshots' }]),
              ...(server.features?.rebuild && can('servers.rebuild') ? [{ id: 'rebuild' as const, label: 'Herbouwen' }] : []),
              ...(server.features?.volumes === false || !can('isos.view') ? [] : [{ id: 'iso' as const, label: 'ISO' }]),
            ]}
            active={tab}
            onChange={setTab}
          />
          {tab === 'overview' && <Overview server={server} live={server.live} features={server.features} linked={linked} onOpen={setTab} />}
          {tab === 'graphs' && <ServerGraphs serverId={server.id} />}
          {tab === 'networks' && <ServerNetworks serverId={server.id} />}
          {tab === 'firewall' && <Firewall server={server} />}
          {tab === 'floating' && <ServerFloatingIps serverId={server.id} />}
          {tab === 'volumes' && <ServerVolumes serverId={server.id} />}
          {tab === 'snapshots' && <Snapshots server={server} />}
          {tab === 'rebuild' && <ServerRebuild serverId={server.id} rebuilding={server.details.rebuilding === true} />}
          {tab === 'iso' && <ServerIso serverId={server.id} />}
        </>
      )}
    </>
  )
}
