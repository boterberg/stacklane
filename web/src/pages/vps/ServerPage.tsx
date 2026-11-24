import { ArrowPathIcon, CameraIcon, EyeIcon, GlobeAltIcon, PlayIcon, PlusIcon, PowerIcon, ServerIcon, StopIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
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
  useRootPassword,
  useSnapshots,
  useVpsServer,
  type FirewallRule,
  type LiveStatus,
  type Snapshot,
  type VpsServer,
} from '../../lib/vps'

type Tab = 'overview' | 'snapshots' | 'firewall'

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

function Overview({ server, live }: { server: VpsServer; live: LiveStatus | null }) {
  const password = useRootPassword(server.id)
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
            {password.data ? (
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
            <Row label={t('Geheugen')}>
              {live.memory === null ? formatBytes(live.max_memory) : t('{used} van {total}', { used: formatBytes(live.memory), total: formatBytes(live.max_memory) })}
            </Row>
            {live.uptime !== null && <Row label={t('Uptime')}>{live.uptime > 0 ? formatDuration(live.uptime * 1000) : '-'}</Row>}
          </dl>
        ) : (
          <p className="text-sm text-zinc-500 dark:text-white/70">{t('De server geeft nu geen gegevens door.')}</p>
        )}
      </Card>
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
            <Field label={t('Omschrijving')}>{(id) => <Input id={id} placeholder={t('Bijvoorbeeld voor de update')} value={description} onChange={(event) => setDescription(event.target.value)} />}</Field>
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
        {confirm?.action === 'rollback'
          ? t('Alles wat na deze snapshot op de server is veranderd, gaat verloren.')
          : t('De snapshot is daarna weg.')}
      </ConfirmDialog>
    </div>
  )
}

const emptyRule: { type: 'in' | 'out'; action: string; proto: string; dport: string; source: string; comment: string } = { type: 'in', action: 'ACCEPT', proto: 'tcp', dport: '', source: '', comment: '' }

function Firewall({ server }: { server: VpsServer }) {
  const { data: rules, isPending } = useFirewall(server.id, true)
  const add = useAddFirewallRule(server.id)
  const remove = useDeleteFirewallRule(server.id)
  const [rule, setRule] = useState(emptyRule)
  const errors = add.error instanceof ApiError ? add.error.fieldErrors : {}

  return (
    <div className="max-w-4xl space-y-6">
      <Card title={t('Regel toevoegen')} description={t('Zonder regels is al het verkeer toegestaan. Regels gelden van boven naar beneden.')}>
        <form
          noValidate
          className="grid gap-4 sm:grid-cols-3"
          onSubmit={(event) => {
            event.preventDefault()
            add.mutate(rule, { onSuccess: () => setRule(emptyRule) })
          }}
        >
          <Field label={t('Richting')}>
            {(id) => (
              <Select id={id} value={rule.type} onChange={(event) => setRule({ ...rule, type: event.target.value as 'in' | 'out' })}>
                <option value="in">{t('Inkomend')}</option>
                <option value="out">{t('Uitgaand')}</option>
              </Select>
            )}
          </Field>
          <Field label={t('Actie')} error={errors.action}>
            {(id) => (
              <Select id={id} value={rule.action} onChange={(event) => setRule({ ...rule, action: event.target.value })}>
                <option value="ACCEPT">{t('Toestaan')}</option>
                <option value="DROP">{t('Blokkeren')}</option>
              </Select>
            )}
          </Field>
          <Field label={t('Protocol')} error={errors.proto}>
            {(id) => (
              <Select id={id} value={rule.proto} onChange={(event) => setRule({ ...rule, proto: event.target.value })}>
                <option value="tcp">TCP</option>
                <option value="udp">UDP</option>
                <option value="icmp">ICMP</option>
              </Select>
            )}
          </Field>
          <Field label={t('Poort')} error={errors.dport}>
            {(id) => <Input id={id} placeholder="22" invalid={!!errors.dport} value={rule.dport} onChange={(event) => setRule({ ...rule, dport: event.target.value })} />}
          </Field>
          <Field label={t('Vanaf IP-adres')} error={errors.source}>
            {(id) => <Input id={id} placeholder={t('Iedereen')} invalid={!!errors.source} value={rule.source} onChange={(event) => setRule({ ...rule, source: event.target.value })} />}
          </Field>
          <Field label={t('Notitie')}>{(id) => <Input id={id} value={rule.comment} onChange={(event) => setRule({ ...rule, comment: event.target.value })} />}</Field>
          <div className="flex justify-end sm:col-span-3">
            <Button type="submit" variant="primary" icon={PlusIcon} disabled={add.isPending}>
              {t('Regel toevoegen')}
            </Button>
          </div>
        </form>
        {errors.server && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{errors.server}</p>}
      </Card>
      {!isPending && (rules ?? []).length === 0 ? (
        <EmptyState title={t('Geen regels')} description={t('Al het verkeer naar deze server is toegestaan.')} />
      ) : (
        <ul className="divide-y divide-zinc-200 rounded-xl border border-zinc-200 bg-white dark:divide-white/10 dark:border-white/10 dark:bg-white/10">
          {(rules ?? []).map((item: FirewallRule) => (
            <li key={item.id} className="flex flex-wrap items-center justify-between gap-4 px-6 py-4 text-sm">
              <div className="flex flex-wrap items-center gap-3">
                <Badge color={item.action === 'ACCEPT' ? 'green' : 'red'}>{item.action === 'ACCEPT' ? t('Toestaan') : t('Blokkeren')}</Badge>
                <span className="text-zinc-800 dark:text-white">
                  {item.type === 'in' ? t('Inkomend') : t('Uitgaand')} · {(item.proto ?? t('alles')).toUpperCase()}
                  {item.dport ? ` · ${t('poort {port}', { port: item.dport })}` : ''}
                </span>
                <span className="text-zinc-500 dark:text-white/60">{item.source ? t('vanaf {source}', { source: item.source }) : t('vanaf iedereen')}</span>
                {item.comment && <span className="text-zinc-400 dark:text-white/50">{item.comment}</span>}
              </div>
              <Button size="sm" icon={TrashIcon} disabled={remove.isPending} onClick={() => remove.mutate(item.id)}>
                {t('Verwijderen')}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

function PowerButtons({ server, live }: { server: VpsServer; live: LiveStatus | null }) {
  const power = usePower(server.id)
  const [stopping, setStopping] = useState(false)
  if (server.status !== 'active' || !live) return null
  const running = live.state === 'running'
  return (
    <div className="flex flex-wrap gap-2">
      {running ? (
        <>
          <Button size="sm" icon={ArrowPathIcon} disabled={power.isPending} onClick={() => power.mutate('reboot')}>
            {t('Herstarten')}
          </Button>
          <Button size="sm" icon={PowerIcon} disabled={power.isPending} onClick={() => power.mutate('shutdown')}>
            {t('Afsluiten')}
          </Button>
          <Button size="sm" icon={StopIcon} disabled={power.isPending} onClick={() => setStopping(true)}>
            {t('Uitzetten')}
          </Button>
        </>
      ) : (
        <Button size="sm" variant="primary" icon={PlayIcon} disabled={power.isPending} onClick={() => power.mutate('start')}>
          {t('Starten')}
        </Button>
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
  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{server.name}</h1>
            <Badge color={serviceStatusColors[server.status]}>{serviceStatusLabels[server.status]}</Badge>
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
        <PowerButtons server={server} live={server.live} />
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
          <Tabs
            tabs={[
              { id: 'overview', label: 'Overzicht' },
              { id: 'snapshots', label: 'Snapshots' },
              { id: 'firewall', label: 'Firewall' },
            ]}
            active={tab}
            onChange={setTab}
          />
          {tab === 'overview' && <Overview server={server} live={server.live} />}
          {tab === 'snapshots' && <Snapshots server={server} />}
          {tab === 'firewall' && <Firewall server={server} />}
        </>
      )}
    </>
  )
}
