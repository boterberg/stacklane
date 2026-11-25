import { ChevronRightIcon, CircleStackIcon, LinkIcon, LinkSlashIcon, MapPinIcon, PlusIcon, ShareIcon } from '@heroicons/react/24/outline'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState, type ComponentType, type ReactNode, type SVGProps } from 'react'
import { Link } from 'react-router'

import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { Select } from '../../components/ui/Field'
import { api, ApiError } from '../../lib/api'
import { useFloatingIpAction, useFloatingIpOptions, useFloatingIps } from '../../lib/floatingIps'
import { t } from '../../lib/i18n'
import { useNetworkServer } from '../../lib/networks'
import { useCan } from '../../lib/vpsAccess'
import { useVolumeAction, useVolumes } from '../../lib/volumes'

type ServerNetwork = { network_id: string; name: string; cidr: string; address: string | null }

function useServerNetworks(serverId: string, enabled: boolean) {
  return useQuery({
    queryKey: ['vps', 'networks', 'server', serverId],
    queryFn: () => api<{ attached: ServerNetwork[]; available: ServerNetwork[] }>(`/vps/servers/${serverId}/networks`),
    enabled,
  })
}

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function Rows({ children }: { children: ReactNode }) {
  return <ul className="divide-y divide-zinc-200 dark:divide-white/10">{children}</ul>
}

function ItemRow({ title, meta, action }: { title: ReactNode; meta?: ReactNode; action?: ReactNode }) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
      <div className="min-w-0">
        <p className="text-sm font-medium text-zinc-800 dark:text-white">{title}</p>
        {meta && <p className="mt-1 font-mono text-xs text-zinc-500 dark:text-white/60">{meta}</p>}
      </div>
      {action}
    </li>
  )
}

function Picker({ label, options, pending, onPick }: { label: string; options: { id: string; label: string }[]; pending: boolean; onPick: (id: string) => void }) {
  const [chosen, setChosen] = useState('')
  const value = options.some((option) => option.id === chosen) ? chosen : (options[0]?.id ?? '')
  if (options.length === 0) return null
  return (
    <form
      noValidate
      className="mt-6 flex flex-wrap items-center gap-3 border-t border-zinc-200 pt-6 dark:border-white/10"
      onSubmit={(event) => {
        event.preventDefault()
        if (value) onPick(value)
      }}
    >
      <div className="min-w-56 flex-1">
        <Select aria-label={label} value={value} onChange={(event) => setChosen(event.target.value)}>
          {options.map((option) => (
            <option key={option.id} value={option.id}>
              {option.label}
            </option>
          ))}
        </Select>
      </div>
      <Button type="submit" variant="primary" icon={LinkIcon} disabled={pending}>
        {label}
      </Button>
    </form>
  )
}

export function ServerVolumes({ serverId }: { serverId: string }) {
  const can = useCan()
  const { data, isPending } = useVolumes()
  const action = useVolumeAction()
  const volumes = data?.volumes ?? []
  const attached = volumes.filter((volume) => volume.server_id === serverId)
  const free = volumes.filter((volume) => volume.status === 'active' && !volume.server_id)
  return (
    <Card
      className="max-w-4xl"
      title={t('Volumes')}
      description={t('Beheer je block storage volumes.')}
      action={
        <ButtonLink size="sm" icon={PlusIcon} to="/vps/volumes">
          {t('Volume bestellen')}
        </ButtonLink>
      }
    >
      {!isPending && attached.length === 0 ? (
        <EmptyState bare title={t('Geen volumes')} description={t('Koppel of bestel een volume om extra opslag te krijgen.')} />
      ) : (
        <Rows>
          {attached.map((volume) => (
            <ItemRow
              key={volume.id}
              title={volume.name}
              meta={volume.size_gb ? t('{size} GB', { size: volume.size_gb }) : undefined}
              action={
                can('volumes.detach') && (
                  <Button size="sm" icon={LinkSlashIcon} disabled={action.isPending} onClick={() => action.mutate({ id: volume.id, action: 'detach' })}>
                    {t('Loskoppelen')}
                  </Button>
                )
              }
            />
          ))}
        </Rows>
      )}
      {can('volumes.attach') && (
        <Picker
          label={t('Volume koppelen')}
          pending={action.isPending}
          options={free.map((volume) => ({ id: volume.id, label: volume.size_gb ? `${volume.name} · ${t('{size} GB', { size: volume.size_gb })}` : volume.name }))}
          onPick={(id) => action.mutate({ id, action: 'attach', serverId })}
        />
      )}
      {errorText(action.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
    </Card>
  )
}

export function ServerFloatingIps({ serverId }: { serverId: string }) {
  const can = useCan()
  const { data, isPending } = useFloatingIps()
  const { data: options } = useFloatingIpOptions(true)
  const action = useFloatingIpAction()
  const integration = options?.servers.find((server) => server.server_id === serverId)?.integration_id
  const floating = data?.floating_ips ?? []
  const attached = floating.filter((ip) => ip.server_id === serverId)
  const movable = floating.filter((ip) => ip.status === 'active' && ip.address && ip.server_id !== serverId && ip.integration_id === integration)
  return (
    <Card
      className="max-w-4xl"
      title={t('Floating IPs')}
      description={t('Vaste IP-adressen die je naar een andere server kunt verplaatsen.')}
      action={
        <ButtonLink size="sm" icon={PlusIcon} to="/vps/floating-ips">
          {t('Floating IP bestellen')}
        </ButtonLink>
      }
    >
      {!isPending && attached.length === 0 ? (
        <EmptyState bare title={t('Geen floating IPs')} description={t('Koppel of bestel een floating IP om aan de slag te gaan.')} />
      ) : (
        <Rows>
          {attached.map((ip) => (
            <ItemRow
              key={ip.id}
              title={ip.address}
              action={
                can('floating_ips.detach') && (
                  <Button size="sm" icon={LinkSlashIcon} disabled={action.isPending} onClick={() => action.mutate({ id: ip.id, action: 'detach' })}>
                    {t('Ontkoppelen')}
                  </Button>
                )
              }
            />
          ))}
        </Rows>
      )}
      {can('floating_ips.attach') && (
        <Picker
          label={t('Floating IP koppelen')}
          pending={action.isPending}
          options={movable.map((ip) => ({
            id: ip.id,
            label: ip.server_name ? t('{address} (nu op {server})', { address: ip.address ?? '', server: ip.server_name }) : (ip.address ?? ''),
          }))}
          onPick={(id) => action.mutate({ id, action: 'attach', serverId })}
        />
      )}
      {errorText(action.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(action.error)}</p>}
    </Card>
  )
}

function NetworkRow({ serverId, network }: { serverId: string; network: ServerNetwork }) {
  const can = useCan()
  const change = useNetworkServer(network.network_id)
  return (
    <ItemRow
      title={
        <Link
          to={`/vps/networks/${network.network_id}`}
          className="underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:decoration-white/20"
        >
          {network.name}
        </Link>
      }
      meta={network.address}
      action={
        can('networks.detach') && (
          <Button size="sm" icon={LinkSlashIcon} disabled={change.isPending} onClick={() => change.mutate({ serverId, attach: false })}>
            {t('Loskoppelen')}
          </Button>
        )
      }
    />
  )
}

function JoinNetwork({ serverId, available }: { serverId: string; available: ServerNetwork[] }) {
  const queryClient = useQueryClient()
  const join = useMutation({
    mutationFn: (networkId: string) => api<void>(`/vps/networks/${networkId}/servers`, { method: 'POST', body: { server_id: serverId } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'networks'] }),
  })
  return (
    <>
      <Picker
        label={t('Aan netwerk koppelen')}
        pending={join.isPending}
        options={available.map((network) => ({ id: network.network_id, label: `${network.name} · ${network.cidr}` }))}
        onPick={(id) => join.mutate(id)}
      />
      {errorText(join.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(join.error)}</p>}
    </>
  )
}

export function ServerNetworks({ serverId }: { serverId: string }) {
  const can = useCan()
  const { data, isPending } = useServerNetworks(serverId, true)
  const attached = data?.attached ?? []
  return (
    <Card
      className="max-w-4xl"
      title={t('Privé netwerken')}
      description={t('Beheer de privénetwerken voor deze server.')}
      action={
        can('networks.create') && (
          <ButtonLink size="sm" icon={PlusIcon} to="/vps/networks">
            {t('Netwerk maken')}
          </ButtonLink>
        )
      }
    >
      {!isPending && attached.length === 0 ? (
        <EmptyState bare title={t("Geen privé IP's")} description={t('Begin door je server aan te sluiten op een privénetwerk.')} />
      ) : (
        <Rows>
          {attached.map((network) => (
            <NetworkRow key={network.network_id} serverId={serverId} network={network} />
          ))}
        </Rows>
      )}
      {can('networks.attach') && data && <JoinNetwork serverId={serverId} available={data.available} />}
    </Card>
  )
}

type Icon = ComponentType<SVGProps<SVGSVGElement>>

function LinkedRow({ icon: Icon, label, value, onOpen }: { icon: Icon; label: string; value: string; onOpen: () => void }) {
  return (
    <li>
      <button type="button" onClick={onOpen} className="group flex w-full items-center gap-3 py-3 text-left first:pt-0">
        <Icon className="size-5 shrink-0 text-zinc-400 dark:text-white/40" aria-hidden />
        <span className="min-w-0 flex-1">
          <span className="block text-sm font-medium text-zinc-800 dark:text-white">{label}</span>
          <span className="block text-sm text-zinc-500 dark:text-white/70">{value}</span>
        </span>
        <ChevronRightIcon className="size-4 text-zinc-400 group-hover:text-zinc-800 dark:text-white/40 dark:group-hover:text-white" aria-hidden />
      </button>
    </li>
  )
}

export function LinkedCard({ serverId, show, onOpen }: { serverId: string; show: { volumes: boolean; floating: boolean; networks: boolean }; onOpen: (tab: 'volumes' | 'floating' | 'networks') => void }) {
  const volumes = useVolumes()
  const floating = useFloatingIps()
  const networks = useServerNetworks(serverId, show.networks)
  const volumeCount = (volumes.data?.volumes ?? []).filter((volume) => volume.server_id === serverId).length
  const floatingCount = (floating.data?.floating_ips ?? []).filter((ip) => ip.server_id === serverId).length
  const networkCount = networks.data?.attached.length ?? 0
  if (!show.volumes && !show.floating && !show.networks) return null
  return (
    <Card title={t('Gekoppeld')}>
      <ul className="divide-y divide-zinc-200 dark:divide-white/10">
        {show.volumes && (
          <LinkedRow
            icon={CircleStackIcon}
            label={t('Volumes')}
            value={volumeCount === 0 ? t('Geen volumes') : volumeCount === 1 ? t('1 volume') : t('{count} volumes', { count: volumeCount })}
            onOpen={() => onOpen('volumes')}
          />
        )}
        {show.floating && (
          <LinkedRow
            icon={MapPinIcon}
            label={t('Floating IPs')}
            value={floatingCount === 0 ? t('Geen floating IPs') : floatingCount === 1 ? t('1 floating IP') : t('{count} floating IPs', { count: floatingCount })}
            onOpen={() => onOpen('floating')}
          />
        )}
        {show.networks && (
          <LinkedRow
            icon={ShareIcon}
            label={t('Netwerken')}
            value={networkCount === 0 ? t('Alleen publiek netwerk') : networkCount === 1 ? t('1 privénetwerk') : t('{count} privénetwerken', { count: networkCount })}
            onOpen={() => onOpen('networks')}
          />
        )}
      </ul>
    </Card>
  )
}
