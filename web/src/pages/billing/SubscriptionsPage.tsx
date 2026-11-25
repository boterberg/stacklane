import { ArrowPathIcon, BanknotesIcon, ClockIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, PerPageSelect, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { expiryText, renewalText, useSubscriptions, type SubscriptionSummary } from '../../lib/billing'
import { formatMoney, formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { subscriptionStatusColors, subscriptionStatuses, subscriptionStatusLabels } from '../../lib/labels'
import { useStoredState } from '../../lib/useStoredState'

type Status = (typeof subscriptionStatuses)[number]

export function SubscriptionTitle({ subscription }: { subscription: SubscriptionSummary }) {
  return (
    <>
      {subscription.name} <span className="font-normal text-zinc-400 dark:text-white/50">#{subscription.code}</span>
    </>
  )
}

function StatusBadge({ status }: { status: string }) {
  return <Badge color={subscriptionStatusColors[status]}>{subscriptionStatusLabels[status]}</Badge>
}

export function SubscriptionsPage() {
  const [query, setQuery] = useState('')
  const [statuses, setStatuses] = useState<Status[]>([])
  const [page, setPage] = useState(1)
  const [view, setView] = useStoredState<ListView>('stacklane-view-subscriptions', 'grid')
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-subscriptions', 5)
  const { data, isPending } = useSubscriptions({ q: query, statuses, page, perPage })
  const subscriptions = data?.data ?? []

  function resetPage<T>(setter: (value: T) => void) {
    return (value: T) => {
      setter(value)
      setPage(1)
    }
  }

  return (
    <>
      <PageHeader title={t('Abonnementen')} />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder={t('Abonnementen zoeken...')} />
        <ToolbarSpacer />
        <MultiSelectFilter
          values={statuses}
          options={[...subscriptionStatuses]}
          labels={subscriptionStatusLabels}
          onChange={resetPage(setStatuses)}
          noun={[t('status'), t('statussen')]}
          placeholder={t('Filter op status...')}
        />
        <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>

      {!isPending && subscriptions.length === 0 ? (
        <EmptyState title={t('Geen abonnementen')} description={t('Je hebt nog geen abonnementen.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {subscriptions.map((subscription) => (
            <RowCard
              key={subscription.id}
              to={`/billing/subscriptions/${subscription.code}`}
              title={<SubscriptionTitle subscription={subscription} />}
              badge={<StatusBadge status={subscription.status} />}
              meta={[
                { icon: ClockIcon, label: expiryText(subscription) },
                { icon: BanknotesIcon, label: t('{price} per maand', { price: formatMoney(subscription.monthly_total_cents) }) },
                { icon: ArrowPathIcon, label: renewalText(subscription) },
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<SubscriptionSummary>
          rows={subscriptions}
          rowKey={(subscription) => subscription.id}
          columns={[
            {
              header: t('Abonnement'),
              cell: (subscription) => (
                <Link to={`/billing/subscriptions/${subscription.code}`} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  {subscription.name}
                </Link>
              ),
            },
            { header: t('Nummer'), cell: (subscription) => `#${subscription.code}` },
            { header: t('Status'), cell: (subscription) => <StatusBadge status={subscription.status} /> },
            { header: t('Per maand'), cell: (subscription) => formatMoney(subscription.monthly_total_cents) },
            { header: t('Verloopt'), cell: (subscription) => formatRelative(subscription.expires_at) },
          ]}
        />
      )}
      {data && subscriptions.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
    </>
  )
}
