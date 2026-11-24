import { CalendarIcon, CurrencyEuroIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, PerPageSelect, SearchField, ToolbarSpacer } from '../../components/ui/ListToolbar'
import { PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { useAdminInvoices } from '../../lib/admin'
import { formatDate, formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'
import { invoiceStatusColors, invoiceStatusLabels } from '../../lib/labels'
import { useStoredState } from '../../lib/useStoredState'

const statuses = ['open', 'paid', 'cancelled', 'refunded']

export function AdminInvoicesPage() {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<string[]>(['open'])
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-admin-invoices', 10)
  const { data, isPending } = useAdminInvoices({ q: query, statuses: filter, page, perPage })
  const invoices = data?.data ?? []

  function resetPage<T>(setter: (value: T) => void) {
    return (value: T) => {
      setter(value)
      setPage(1)
    }
  }

  return (
    <>
      <PageHeader title={t('Alle facturen')} description={t('Facturen van al je klanten. Zet een overboeking hier op betaald.')} />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder={t('Zoek op nummer of klant...')} />
        <ToolbarSpacer />
        <MultiSelectFilter
          values={filter}
          options={statuses}
          labels={invoiceStatusLabels}
          onChange={resetPage(setFilter)}
          noun={[t('status'), t('statussen')]}
          placeholder={t('Filter op status...')}
        />
        <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />
      </ListToolbar>

      {!isPending && invoices.length === 0 ? (
        <EmptyState title={t('Geen facturen')} description={t('Er zijn geen facturen gevonden.')} />
      ) : (
        <RowCardList>
          {invoices.map((invoice) => (
            <RowCard
              key={invoice.id}
              to={`/admin/invoices/${invoice.id}`}
              title={`#${invoice.id}`}
              badge={<Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge>}
              meta={[
                { icon: UserIcon, label: invoice.customer_name },
                { icon: CalendarIcon, label: formatDate(invoice.issued_at) },
                { icon: CurrencyEuroIcon, label: formatEuro(invoice.total_cents).replace('€', '').trim() },
              ]}
            />
          ))}
        </RowCardList>
      )}
      {data && invoices.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
    </>
  )
}
