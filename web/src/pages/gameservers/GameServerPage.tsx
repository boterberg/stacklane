import { ArrowPathIcon, CheckIcon, ClipboardDocumentIcon, PlayIcon, StopIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Tabs } from '../../components/ui/Tabs'
import { formatBytes, formatDuration, formatMegabytes } from '../../lib/format'
import { useGameServer, usePower, type GameServerDetail } from '../../lib/gameservers'
import { powerStateColors, powerStateLabels, serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { ServerActivity } from './ServerActivity'
import { ServerBackups } from './ServerBackups'
import { ServerConsole } from './ServerConsole'
import { ServerFiles } from './ServerFiles'
import { t } from '../../lib/i18n'

type Tab = 'console' | 'files' | 'backups' | 'activity'

function Address({ value }: { value: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      type="button"
      onClick={async () => {
        await navigator.clipboard.writeText(value)
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      }}
      className="flex items-center gap-2 rounded-md font-mono text-sm text-zinc-500 hover:text-zinc-800 dark:text-white/70 dark:hover:text-white"
      title={t('Kopiëren')}
    >
      {value}
      {copied ? <CheckIcon className="size-4" /> : <ClipboardDocumentIcon className="size-4" />}
    </button>
  )
}

function Meter({ label, value, detail, percent }: { label: string; value: string; detail?: string; percent?: number }) {
  return (
    <div className="rounded-xl border border-zinc-200 bg-zinc-50 px-5 py-4 dark:border-white/7 dark:bg-white/5">
      <p className="text-sm text-zinc-500 dark:text-white/70">{label}</p>
      <p className="mt-1 text-lg font-medium text-zinc-800 dark:text-white">
        {value}
        {detail && <span className="ml-1 text-sm font-normal text-zinc-500 dark:text-white/60">{detail}</span>}
      </p>
      {percent !== undefined && (
        <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-zinc-200 dark:bg-white/10">
          <div className="accent-bar h-full rounded-full bg-indigo-500" style={{ width: `${Math.min(100, Math.max(2, percent))}%` }} />
        </div>
      )}
    </div>
  )
}

function Resources({ server }: { server: GameServerDetail }) {
  const live = server.resources?.resources
  const memoryLimit = server.details.memory ?? 0
  const memoryUsed = live ? live.memory_bytes / 1024 / 1024 : 0
  const cpuLimit = server.details.cpu ?? 100
  return (
    <div className="mb-8 grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
      <Meter label={t('Processor')} value={live ? `${Math.round(live.cpu_absolute)}%` : t('n.v.t.')} detail={t('van {limit}', { limit: `${cpuLimit}%` })} percent={live ? (live.cpu_absolute / cpuLimit) * 100 : 0} />
      <Meter
        label={t('Geheugen')}
        value={live ? formatMegabytes(Math.round(memoryUsed)) : t('n.v.t.')}
        detail={memoryLimit ? t('van {limit}', { limit: formatMegabytes(memoryLimit) }) : undefined}
        percent={memoryLimit ? (memoryUsed / memoryLimit) * 100 : 0}
      />
      <Meter label={t('Opslag')} value={live ? formatBytes(live.disk_bytes) : t('n.v.t.')} detail={server.details.disk ? t('van {limit}', { limit: formatMegabytes(server.details.disk) }) : undefined} />
      <Meter label={t('Online')} value={live && live.uptime > 0 ? formatDuration(live.uptime) : t('n.v.t.')} />
    </div>
  )
}

export function GameServerPage() {
  const { id = '' } = useParams()
  const { data: server, isPending, error } = useGameServer(id)
  const power = usePower(id)
  const [tab, setTab] = useState<Tab>('console')

  if (isPending) return null
  if (error || !server) {
    return (
      <EmptyState
        title={t('Server niet gevonden')}
        description={t('Deze server bestaat niet of hoort niet bij je account.')}
        action={
          <ButtonLink to="/gameservers/servers" size="sm">
            {t('Naar je servers')}
          </ButtonLink>
        }
      />
    )
  }

  const state = server.resources?.current_state ?? 'offline'
  const ready = server.status === 'active' && !server.install_state

  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{server.name}</h1>
            {server.status !== 'active' ? (
              <Badge color={serviceStatusColors[server.status]}>{serviceStatusLabels[server.status]}</Badge>
            ) : server.install_state ? (
              <Badge color="sky">{t('Wordt geïnstalleerd')}</Badge>
            ) : (
              <Badge color={powerStateColors[state]}>{powerStateLabels[state]}</Badge>
            )}
          </div>
          <div className="mt-2 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            {server.details.address && <Address value={server.details.address} />}
            {server.details.egg && <span>{server.details.egg}</span>}
            {server.product_name && <span>{server.product_name}</span>}
          </div>
        </div>
        {ready && (
          <div className="flex gap-2">
            <Button size="sm" icon={PlayIcon} disabled={power.isPending || state !== 'offline'} onClick={() => power.mutate('start')}>
              {t('Starten')}
            </Button>
            <Button size="sm" icon={ArrowPathIcon} disabled={power.isPending || state === 'offline'} onClick={() => power.mutate('restart')}>
              {t('Herstarten')}
            </Button>
            <Button size="sm" icon={StopIcon} disabled={power.isPending || state === 'offline'} onClick={() => power.mutate(state === 'stopping' ? 'kill' : 'stop')}>
              {state === 'stopping' ? t('Forceren') : t('Stoppen')}
            </Button>
          </div>
        )}
      </div>

      {server.status === 'failed' && (
        <EmptyState title={t('Aanmaken mislukt')} description={t('Er ging iets mis bij het aanmaken van je server. Ons supportteam is op de hoogte.')} />
      )}
      {(server.status === 'pending' || server.status === 'provisioning') && (
        <EmptyState title={t('Je server wordt aangemaakt')} description={t('Dit duurt meestal minder dan een minuut. De pagina ververst vanzelf.')} />
      )}
      {server.status === 'suspended' && <EmptyState title={t('Server opgeschort')} description={t('Neem contact op met support om je server weer te activeren.')} />}

      {server.status === 'active' && (
        <>
          <Resources server={server} />
          <Tabs
            tabs={[
              { id: 'console', label: 'Console' },
              { id: 'files', label: 'Bestanden' },
              { id: 'backups', label: 'Back-ups' },
              { id: 'activity', label: 'Activiteit' },
            ]}
            active={tab}
            onChange={setTab}
          />
          {tab === 'console' && <ServerConsole serverId={server.id} />}
          {tab === 'files' && <ServerFiles serverId={server.id} />}
          {tab === 'backups' && <ServerBackups serverId={server.id} />}
          {tab === 'activity' && <ServerActivity serverId={server.id} />}
        </>
      )}
    </>
  )
}
