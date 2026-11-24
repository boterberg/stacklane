import { CalendarIcon, CubeIcon, PlusIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { useWebsites, type Website } from '../../lib/webhosting'

export function WebhostingPage() {
  const { data: websites, isPending } = useWebsites()
  const [query, setQuery] = useState('')
  const [view, setView] = useState<ListView>('grid')
  const needle = query.trim().toLowerCase()
  const shown = (websites ?? []).filter((website) => !needle || website.name.includes(needle) || website.product_name?.toLowerCase().includes(needle))
  const badge = (website: Website) => <Badge color={serviceStatusColors[website.status]}>{serviceStatusLabels[website.status]}</Badge>

  return (
    <>
      <PageHeader
        title={t('Webhosting')}
        actions={
          <ButtonLink to="/webhosting/order" size="sm" icon={PlusIcon}>
            {t('Webhosting bestellen')}
          </ButtonLink>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Webhosting zoeken...')} />
        <ToolbarSpacer />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen webhosting')} description={t('Geen webhostingpakketten gevonden.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((website) => (
            <RowCard
              key={website.id}
              to={`/webhosting/${website.id}`}
              title={website.name}
              badge={badge(website)}
              meta={[
                ...(website.product_name ? [{ icon: CubeIcon, label: website.product_name }] : []),
                ...(website.details.username ? [{ icon: UserIcon, label: website.details.username }] : []),
                ...(website.expires_at ? [{ icon: CalendarIcon, label: t('Verloopt op {date}', { date: formatDate(website.expires_at) }) }] : []),
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<Website>
          rows={shown}
          rowKey={(website) => website.id}
          columns={[
            {
              header: t('Domein'),
              cell: (website) => (
                <Link to={`/webhosting/${website.id}`} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  {website.name}
                </Link>
              ),
            },
            { header: t('Status'), cell: badge },
            { header: t('Pakket'), cell: (website) => website.product_name ?? '-' },
            { header: t('Verloopt'), cell: (website) => (website.expires_at ? formatDate(website.expires_at) : '-') },
          ]}
        />
      )}
    </>
  )
}
