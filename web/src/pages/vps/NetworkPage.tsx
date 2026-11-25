import { GlobeAltIcon, InformationCircleIcon, LinkIcon, LinkSlashIcon, ServerStackIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { useDeleteNetwork, useNetwork, useNetworkServer, useRenameNetwork, type NetworkDetail } from '../../lib/networks'
import { useCan } from '../../lib/vpsAccess'

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function Servers({ network }: { network: NetworkDetail }) {
  const can = useCan()
  const change = useNetworkServer(network.id)
  const [serverId, setServerId] = useState('')
  const chosen = network.candidates.some((server) => server.server_id === serverId) ? serverId : (network.candidates[0]?.server_id ?? '')

  return (
    <Card title={t('Servers')} description={t('Servers in dit netwerk krijgen een extra netwerkkaart met een privé IP-adres.')}>
      {network.restart_needed && network.members.length > 0 && (
        <div className="mb-6 flex gap-3 rounded-lg border border-blue-200 bg-blue-50 px-4 py-3 text-sm text-blue-800 dark:border-blue-400/20 dark:bg-blue-400/10 dark:text-blue-300">
          <InformationCircleIcon className="size-5 shrink-0" aria-hidden />
          <p>{t('Na het koppelen staat het adres klaar voor de volgende herstart. Herstart de server, of stel het adres zelf in op de nieuwe netwerkkaart.')}</p>
        </div>
      )}
      {network.members.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Er zitten nog geen servers in dit netwerk.')}</p>
      ) : (
        <ul className="divide-y divide-zinc-200 dark:divide-white/10">
          {network.members.map((member) => (
            <li key={member.server_id} className="flex flex-wrap items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0">
                <Link
                  to={`/vps/servers/${member.server_id}`}
                  className="text-sm font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20"
                >
                  {member.server_name}
                </Link>
                <p className="mt-1 font-mono text-xs text-zinc-500 dark:text-white/60">{member.address}</p>
              </div>
              {can('networks.detach') && (
                <Button size="sm" icon={LinkSlashIcon} disabled={change.isPending} onClick={() => change.mutate({ serverId: member.server_id, attach: false })}>
                  {t('Loskoppelen')}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      {can('networks.attach') && (
        <div className="mt-6 border-t border-zinc-200 pt-6 dark:border-white/10">
          {network.candidates.length === 0 ? (
            <p className="text-sm text-zinc-500 dark:text-white/60">{t('Je hebt geen andere actieve server op dit platform die je kunt koppelen.')}</p>
          ) : (
            <form
              noValidate
              className="flex flex-wrap items-end gap-3"
              onSubmit={(event) => {
                event.preventDefault()
                change.mutate({ serverId: chosen, attach: true })
              }}
            >
              <div className="min-w-56 flex-1">
                <Field label={t('Server koppelen')}>
                  {(id) => (
                    <Select id={id} value={chosen} onChange={(event) => setServerId(event.target.value)}>
                      {network.candidates.map((server) => (
                        <option key={server.server_id} value={server.server_id}>
                          {server.server_name}
                        </option>
                      ))}
                    </Select>
                  )}
                </Field>
              </div>
              <Button type="submit" variant="primary" icon={LinkIcon} disabled={!chosen || change.isPending}>
                {change.isPending ? t('Bezig...') : t('Koppelen')}
              </Button>
            </form>
          )}
        </div>
      )}
      {errorText(change.error) && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{errorText(change.error)}</p>}
    </Card>
  )
}

function Settings({ network }: { network: NetworkDetail }) {
  const [name, setName] = useState(network.name)
  const rename = useRenameNetwork(network.id)
  const errors = rename.error instanceof ApiError ? rename.error.fieldErrors : {}
  return (
    <Card title={t('Instellingen')}>
      <form
        noValidate
        className="space-y-6"
        onSubmit={(event) => {
          event.preventDefault()
          rename.mutate(name.trim())
        }}
      >
        <Field label={t('Naam')} error={errors.name}>
          {(id) => <Input id={id} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>
        <div className="flex justify-end">
          <Button type="submit" variant="primary" disabled={rename.isPending || name.trim() === network.name}>
            {t('Opslaan')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

export function NetworkPage() {
  const { id = '' } = useParams()
  const { data: network, isPending, error } = useNetwork(id)
  const can = useCan()
  const remove = useDeleteNetwork()
  const navigate = useNavigate()
  const [confirm, setConfirm] = useState(false)

  if (isPending) return null
  if (error || !network) {
    return (
      <EmptyState
        title={t('Netwerk niet gevonden')}
        description={t('Dit netwerk bestaat niet of is van iemand anders.')}
        action={
          <ButtonLink to="/vps/networks" size="sm">
            {t('Naar netwerken')}
          </ButtonLink>
        }
      />
    )
  }

  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{network.name}</h1>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            <span className="flex items-center gap-2">
              <GlobeAltIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {network.cidr}
            </span>
            <span className="flex items-center gap-2">
              <ServerStackIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {network.servers === 1 ? t('1 server') : t('{count} servers', { count: network.servers })}
            </span>
          </div>
        </div>
        {can('networks.delete') && (
          <Button size="sm" icon={TrashIcon} onClick={() => setConfirm(true)}>
            {t('Verwijderen')}
          </Button>
        )}
      </div>

      <div className="max-w-4xl space-y-6">
        <Servers network={network} />
        {can('networks.update') && <Settings key={network.name} network={network} />}
      </div>

      <ConfirmDialog
        open={confirm}
        title={t('Netwerk verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => {
          remove.reset()
          setConfirm(false)
        }}
        onConfirm={() => remove.mutate(id, { onSuccess: () => navigate('/vps/networks') })}
      >
        {errorText(remove.error) ?? t('Het netwerk en het IP-bereik verdwijnen. Dit kan alleen als er geen servers meer in zitten.')}
      </ConfirmDialog>
    </>
  )
}
