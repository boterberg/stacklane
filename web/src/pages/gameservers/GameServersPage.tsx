import { CpuChipIcon, GlobeAltIcon, PuzzlePieceIcon, ServerIcon } from '@heroicons/react/24/outline'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatMegabytes } from '../../lib/format'
import { useGameServers } from '../../lib/gameservers'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'

export function GameServersPage() {
  const { data: servers, isPending } = useGameServers()

  return (
    <>
      <PageHeader title="Servers" />
      {!isPending && (servers ?? []).length === 0 ? (
        <EmptyState title="Geen servers" description="Je hebt nog geen gameservers." />
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
                ...(server.details.memory ? [{ icon: CpuChipIcon, label: `${formatMegabytes(server.details.memory)} geheugen` }] : []),
                ...(server.product_name ? [{ icon: ServerIcon, label: server.product_name }] : []),
              ]}
            />
          ))}
        </RowCardList>
      )}
    </>
  )
}
