import { CpuChipIcon, GlobeAltIcon, PlusIcon, ServerIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField } from '../../components/ui/ListToolbar'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { t } from '../../lib/i18n'
import { useVpsAccess } from '../../lib/vpsAccess'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { specsLine, useVpsServers } from '../../lib/vps'

export function VpsServersPage() {
  const { data: servers, isPending } = useVpsServers()
  const [query, setQuery] = useState('')
  const needle = query.trim().toLowerCase()
  const shown = (servers ?? []).filter((server) => !needle || server.name.toLowerCase().includes(needle) || server.details.ipv4?.includes(needle))
  const isOwner = useVpsAccess().data?.is_owner !== false
  const order = isOwner ? (
    <ButtonLink to="/vps/servers/order" size="sm" icon={PlusIcon}>
      {t('Server bestellen')}
    </ButtonLink>
  ) : null

  return (
    <>
      <PageHeader title={t('Servers')} actions={order} />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Servers zoeken...')} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen servers')} description={needle ? t('Er zijn geen servers gevonden.') : t('Bestel een server om te beginnen.')} />
      ) : (
        <RowCardList>
          {shown.map((server) => (
            <RowCard
              key={server.id}
              to={`/vps/servers/${server.id}`}
              title={server.name}
              badge={<Badge color={serviceStatusColors[server.status]}>{serviceStatusLabels[server.status]}</Badge>}
              meta={[
                ...(server.details.ipv4 ? [{ icon: GlobeAltIcon, label: server.details.ipv4 }] : []),
                ...(server.details.image ? [{ icon: ServerIcon, label: server.details.image }] : []),
                ...(server.details.cores ? [{ icon: CpuChipIcon, label: specsLine(server.details) }] : []),
              ]}
            />
          ))}
        </RowCardList>
      )}
    </>
  )
}
