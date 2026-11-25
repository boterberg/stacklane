import { BuildingOfficeIcon, CalendarIcon, CreditCardIcon, DocumentTextIcon, EnvelopeIcon, ServerStackIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, PerPageSelect, SearchField, ToolbarSpacer } from '../../components/ui/ListToolbar'
import { PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { useCustomers } from '../../lib/admin'
import { formatDate, formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useStoredState } from '../../lib/useStoredState'

export function CustomersPage() {
  const [query, setQuery] = useState('')
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-admin-customers', 25)
  const { data, isPending } = useCustomers({ q: query, page, perPage })
  const customers = data?.data ?? []

  function resetPage<T>(setter: (value: T) => void) {
    return (value: T) => {
      setter(value)
      setPage(1)
    }
  }

  return (
    <>
      <PageHeader title={t('Klanten')} description={t('Al je klanten met hun abonnementen, diensten en openstaande facturen.')} />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder={t('Zoek op naam, bedrijf of e-mailadres...')} />
        <ToolbarSpacer />
        <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />
      </ListToolbar>
      {!isPending && customers.length === 0 ? (
        <EmptyState title={t('Geen klanten')} description={t('Er zijn geen klanten gevonden.')} />
      ) : (
        <RowCardList>
          {customers.map((customer) => (
            <RowCard
              key={customer.id}
              to={`/admin/customers/${customer.id}`}
              title={customer.name}
              badge={customer.open_invoices > 0 ? <Badge color="amber">{t('{count} open', { count: customer.open_invoices })}</Badge> : undefined}
              meta={[
                { icon: EnvelopeIcon, label: customer.email },
                ...(customer.company_name ? [{ icon: BuildingOfficeIcon, label: customer.company_name }] : []),
                { icon: CreditCardIcon, label: customer.active_subscriptions === 1 ? t('1 abonnement') : t('{count} abonnementen', { count: customer.active_subscriptions }) },
                { icon: ServerStackIcon, label: customer.active_services === 1 ? t('1 dienst') : t('{count} diensten', { count: customer.active_services }) },
                ...(customer.open_cents > 0 ? [{ icon: DocumentTextIcon, label: t('{amount} openstaand', { amount: formatEuro(customer.open_cents) }) }] : []),
                { icon: CalendarIcon, label: t('Klant sinds {date}', { date: formatDate(customer.created_at) }) },
              ]}
            />
          ))}
        </RowCardList>
      )}
      {data && customers.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
    </>
  )
}
