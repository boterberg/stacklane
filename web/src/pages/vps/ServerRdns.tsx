import { PencilSquareIcon } from '@heroicons/react/24/outline'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Input } from '../../components/ui/Field'
import { DataTable } from '../../components/ui/ResourceList'
import { api, ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { useCan } from '../../lib/vpsAccess'

type Address = { ip: string; ptr: string | null; editable: boolean }

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

function Editor({ serverId, address, onDone }: { serverId: string; address: Address; onDone: () => void }) {
  const queryClient = useQueryClient()
  const [hostname, setHostname] = useState(address.ptr ?? '')
  const save = useMutation({
    mutationFn: () => api<void>(`/vps/servers/${serverId}/rdns`, { method: 'PUT', body: { ip: address.ip, hostname: hostname.trim() || null } }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['vps', 'rdns', serverId] })
      onDone()
    },
  })
  return (
    <form
      noValidate
      className="flex flex-wrap items-start gap-2"
      onSubmit={(event) => {
        event.preventDefault()
        save.mutate()
      }}
    >
      <div className="min-w-56 flex-1">
        <Input aria-label={t('PTR record')} placeholder="mail.jouwdomein.nl" value={hostname} invalid={!!save.error} onChange={(event) => setHostname(event.target.value)} />
        {errorText(save.error) && <p className="mt-2 text-sm font-medium text-red-500 dark:text-red-400">{errorText(save.error)}</p>}
      </div>
      <Button type="submit" variant="primary" disabled={save.isPending}>
        {save.isPending ? t('Bezig...') : t('Opslaan')}
      </Button>
      <Button variant="ghost" onClick={onDone}>
        {t('Annuleren')}
      </Button>
    </form>
  )
}

export function ServerRdns({ serverId }: { serverId: string }) {
  const can = useCan()
  const [editing, setEditing] = useState<string | null>(null)
  const { data: addresses = [], isPending } = useQuery({
    queryKey: ['vps', 'rdns', serverId],
    queryFn: () => api<Address[]>(`/vps/servers/${serverId}/rdns`),
  })
  const editable = addresses.some((address) => address.editable)

  return (
    <Card className="max-w-4xl" title={t('Reverse DNS')} description={t('Beheer PTR-records voor je server IP-adressen. De hostnaam moet eerst met een A-record naar het IP-adres wijzen.')}>
      {!isPending && !editable && (
        <p className="mb-6 text-sm text-zinc-500 dark:text-white/60">{t('Reverse DNS stel je voor deze IP-adressen in via support.')}</p>
      )}
      <DataTable<Address>
        rows={addresses}
        rowKey={(address) => address.ip}
        columns={[
          { header: t('IP-adres'), cell: (address) => <span className="font-mono">{address.ip}</span> },
          { header: t('Type'), cell: () => <Badge color="blue">IPv4</Badge> },
          {
            header: t('PTR record'),
            cell: (address) =>
              editing === address.ip ? (
                <Editor serverId={serverId} address={address} onDone={() => setEditing(null)} />
              ) : (
                (address.ptr ?? <span className="text-zinc-500 dark:text-white/60">{t('Niet ingesteld')}</span>)
              ),
          },
          {
            header: '',
            cell: (address) =>
              address.editable && can('servers.rdns') && editing !== address.ip ? (
                <Button size="sm" icon={PencilSquareIcon} onClick={() => setEditing(address.ip)}>
                  {t('Wijzigen')}
                </Button>
              ) : null,
          },
        ]}
      />
    </Card>
  )
}
