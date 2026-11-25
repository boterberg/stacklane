import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { ArrowDownTrayIcon, BanknotesIcon, CalendarIcon, EllipsisHorizontalIcon, EyeIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, PerPageSelect, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { invoicePdfUrl, useInvoices, type InvoiceSummary } from '../../lib/billing'
import { formatDate, formatMoney } from '../../lib/format'
import { t } from '../../lib/i18n'
import { invoiceStatusColors, invoiceStatusLabels } from '../../lib/labels'
import { useStoredState } from '../../lib/useStoredState'

const menuItem = 'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-zinc-800 data-focus:bg-zinc-50 dark:text-white dark:data-focus:bg-zinc-600'

function InvoiceMenu({ invoice }: { invoice: InvoiceSummary }) {
  return (
    <Menu>
      <MenuButton className="rounded-md p-1.5 text-zinc-500 hover:bg-zinc-800/5 hover:text-zinc-800 dark:text-white/70 dark:hover:bg-white/10 dark:hover:text-white" aria-label={t('Acties')}>
        <EllipsisHorizontalIcon className="size-5" />
      </MenuButton>
      <MenuItems anchor={{ to: 'bottom end', gap: 4 }} className="z-50 min-w-44 rounded-lg border border-zinc-200 bg-white p-1 shadow-xs focus:outline-none dark:border-zinc-600 dark:bg-zinc-700">
        <MenuItem>
          <Link to={`/billing/invoices/${invoice.id}`} className={menuItem}>
            <EyeIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
            {t('Bekijken')}
          </Link>
        </MenuItem>
        <MenuItem>
          <a href={invoicePdfUrl(invoice.id)} className={menuItem}>
            <ArrowDownTrayIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
            {t('Downloaden')}
          </a>
        </MenuItem>
      </MenuItems>
    </Menu>
  )
}

export function InvoicesPage() {
  const [query, setQuery] = useState('')
  const [page, setPage] = useState(1)
  const [view, setView] = useStoredState<ListView>('stacklane-view-invoices', 'grid')
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-invoices', 5)
  const { data, isPending } = useInvoices({ q: query, page, perPage })
  const invoices = data?.data ?? []

  return (
    <>
      <PageHeader title={t('Facturen')} />
      <ListToolbar>
        <SearchField
          value={query}
          onChange={(value) => {
            setQuery(value)
            setPage(1)
          }}
          placeholder={t('Facturen zoeken...')}
        />
        <ToolbarSpacer />
        <PerPageSelect
          value={perPage}
          onChange={(value) => {
            setPerPage(value)
            setPage(1)
          }}
        />
        <ViewToggle value={view} onChange={setView} />
      </ListToolbar>

      {!isPending && invoices.length === 0 ? (
        <EmptyState title={t('Geen facturen')} description={t('Je hebt nog geen facturen.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {invoices.map((invoice) => (
            <RowCard
              key={invoice.id}
              to={`/billing/invoices/${invoice.id}`}
              title={`#${invoice.id}`}
              badge={<Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge>}
              meta={[
                { icon: CalendarIcon, label: formatDate(invoice.issued_at) },
                { icon: BanknotesIcon, label: formatMoney(invoice.total_cents, invoice.currency) },
              ]}
              aside={<InvoiceMenu invoice={invoice} />}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<InvoiceSummary>
          rows={invoices}
          rowKey={(invoice) => String(invoice.id)}
          columns={[
            {
              header: t('Nummer'),
              cell: (invoice) => (
                <Link to={`/billing/invoices/${invoice.id}`} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  #{invoice.id}
                </Link>
              ),
            },
            { header: t('Status'), cell: (invoice) => <Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge> },
            { header: t('Factuurdatum'), cell: (invoice) => formatDate(invoice.issued_at) },
            { header: t('Vervaldatum'), cell: (invoice) => formatDate(invoice.due_at) },
            { header: t('Bedrag'), cell: (invoice) => formatMoney(invoice.total_cents, invoice.currency) },
            { header: '', cell: (invoice) => <InvoiceMenu invoice={invoice} />, className: 'w-12 text-right' },
          ]}
        />
      )}
      {data && invoices.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
    </>
  )
}
