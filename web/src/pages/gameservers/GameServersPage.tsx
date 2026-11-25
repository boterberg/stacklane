import { CpuChipIcon, GlobeAltIcon, PuzzlePieceIcon, ServerIcon, UserGroupIcon } from '@heroicons/react/24/outline'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatMegabytes } from '../../lib/format'
import { useGameServers } from '../../lib/gameservers'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { t } from '../../lib/i18n'

export function GameServersPage() {
  const { data: servers, isPending } = useGameServers()

  return (
    <>
      <PageHeader title={t('Servers')} />
      {!isPending && (servers ?? []).length === 0 ? (
        <EmptyState title={t('Geen servers')} description={t('Je hebt nog geen gameservers.')} />
      ) : (
        <RowCardList>
          {(servers ?? []).map((server) => (
            <RowCard
              key={server.id}
              to={`/gameservers/servers/${server.id}`}
              title={server.name}
              badge={<Badge color={serviceStatusColors[server.status]}>{serviceStatusLabels[server.status]}</Badge>}
              meta={[
                ...(server.details.address ? [{ icon: GlobeAltIcon, label: server.details.address }] : []),
                ...(server.details.egg ? [{ icon: PuzzlePieceIcon, label: server.details.egg }] : []),
                ...(server.details.memory ? [{ icon: CpuChipIcon, label: t('{amount} geheugen', { amount: formatMegabytes(server.details.memory) }) }] : []),
                ...(server.product_name ? [{ icon: ServerIcon, label: server.product_name }] : []),
                ...(server.owner_name ? [{ icon: UserGroupIcon, label: t('Gedeeld door {name}', { name: server.owner_name }) }] : []),
              ]}
            />
          ))}
        </RowCardList>
      )}
    </>
  )
}
