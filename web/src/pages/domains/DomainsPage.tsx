import { CalendarIcon, GlobeAltIcon, PlusIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { useDomains, type Domain, type DomainStatus } from '../../lib/domains'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'

const statuses: DomainStatus[] = ['active', 'pending', 'provisioning', 'suspended', 'failed']

function expires(domain: Domain) {
  const date = domain.details.expires_on ?? domain.expires_at
  return date ? t('Verloopt op {date}', { date: formatDate(date) }) : t('Nog niet geregistreerd')
}

export function DomainsPage() {
  const { data: domains, isPending } = useDomains()
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<DomainStatus[]>([])
  const [view, setView] = useState<ListView>('grid')
  const needle = query.trim().toLowerCase()
  const shown = (domains ?? []).filter((domain) => (!needle || domain.name.includes(needle)) && (filter.length === 0 || filter.includes(domain.status)))
  const badge = (domain: Domain) => <Badge color={serviceStatusColors[domain.status]}>{serviceStatusLabels[domain.status]}</Badge>

  return (
    <>
      <PageHeader
        title={t('Domeinen')}
        actions={
          <ButtonLink to="/domains/register" size="sm" icon={PlusIcon}>
            {t('Domein registreren')}
          </ButtonLink>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('Zoek domeinen...')} />
        <ToolbarSpacer />
        <MultiSelectFilter
          values={filter}
          options={statuses}
          labels={serviceStatusLabels as Record<DomainStatus, string>}
          onChange={setFilter}
          noun={[t('status'), t('statussen')]}
          placeholder={t('Filter op status...')}
        />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen domeinen')} description={needle || filter.length > 0 ? t('Er zijn geen domeinen gevonden.') : t('Je bezit geen domeinen.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {shown.map((domain) => (
            <RowCard
              key={domain.id}
              to={`/domains/${domain.id}`}
              title={domain.name}
              badge={badge(domain)}
              meta={[
                { icon: CalendarIcon, label: expires(domain) },
                ...(domain.details.nameservers?.length ? [{ icon: ServerStackIcon, label: domain.details.nameservers[0] }] : []),
                ...(domain.details.operation === 'transfer' ? [{ icon: GlobeAltIcon, label: t('Verhuisd') }] : []),
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<Domain>
          rows={shown}
          rowKey={(domain) => domain.id}
          columns={[
            {
              header: t('Domein'),
              cell: (domain) => (
                <Link to={`/domains/${domain.id}`} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  {domain.name}
                </Link>
              ),
            },
            { header: t('Status'), cell: badge },
            { header: t('Verloopt'), cell: (domain) => (domain.details.expires_on ? formatDate(domain.details.expires_on) : '-') },
            { header: t('Nameservers'), cell: (domain) => domain.details.nameservers?.join(', ') ?? '-' },
          ]}
        />
      )}
    </>
  )
}
