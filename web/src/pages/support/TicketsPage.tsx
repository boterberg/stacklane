import { CalendarIcon, ChatBubbleLeftRightIcon, HashtagIcon, InboxIcon, PlusIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link, useSearchParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, PerPageSelect, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatRelative } from '../../lib/format'
import { defaultTicketStatuses, ticketStatusColors, ticketStatuses, ticketStatusLabels, type TicketStatus } from '../../lib/labels'
import { ticketPath, useTickets, type TicketScope, type TicketSummary } from '../../lib/support'
import { useStoredState } from '../../lib/useStoredState'
import { CreateTicketPanel } from './CreateTicketPanel'
import { t } from '../../lib/i18n'

function StatusBadge({ status }: { status: TicketStatus }) {
  return <Badge color={ticketStatusColors[status]}>{ticketStatusLabels[status]}</Badge>
}

export function TicketsPage({ scope = 'customer' }: { scope?: TicketScope }) {
  const staff = scope === 'staff'
  const [searchParams] = useSearchParams()
  const [creating, setCreating] = useState(searchParams.get('new') === '1')
  const [query, setQuery] = useState('')
  const [statuses, setStatuses] = useState<TicketStatus[]>(defaultTicketStatuses)
  const [page, setPage] = useState(1)
  const [view, setView] = useStoredState<ListView>(`stacklane-view-tickets-${scope}`, 'grid')
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-tickets', 5)
  const { data, isPending } = useTickets(scope, { q: query, statuses, page, perPage })

  function resetPage<T>(setter: (value: T) => void) {
    return (value: T) => {
      setter(value)
      setPage(1)
    }
  }

  const tickets = data?.data ?? []

  return (
    <>
      <PageHeader
        title={staff ? t('Alle tickets') : t('Tickets')}
        actions={
          staff ? undefined : (
            <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
              {t('Ticket maken')}
            </Button>
          )
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder={t('Zoek tickets...')} />
        <ToolbarSpacer />
        <MultiSelectFilter
          values={statuses}
          options={ticketStatuses}
          labels={ticketStatusLabels}
          onChange={resetPage(setStatuses)}
          noun={[t('status'), t('statussen')]}
          placeholder={t('Filter op status...')}
        />
        {tickets.length > 0 && <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />}
        <ViewToggle value={view} onChange={resetPage(setView)} />
      </ListToolbar>

      {!isPending && tickets.length === 0 ? (
        <EmptyState title={t('Geen tickets')} description={t('Er zijn geen tickets gevonden.')} />
      ) : view === 'grid' ? (
        <RowCardList>
          {tickets.map((ticket) => (
            <RowCard
              key={ticket.id}
              to={ticketPath(scope, ticket.number)}
              title={ticket.subject}
              badge={<StatusBadge status={ticket.status} />}
              meta={[
                { icon: HashtagIcon, label: ticket.number },
                ...(staff ? [{ icon: UserIcon, label: ticket.customer_name }] : []),
                { icon: InboxIcon, label: ticket.department },
                { icon: ChatBubbleLeftRightIcon, label: ticket.message_count === 1 ? t('1 bericht') : t('{count} berichten', { count: ticket.message_count }) },
                { icon: CalendarIcon, label: t('Laatste reactie {when}', { when: formatRelative(ticket.last_reply_at) }) },
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<TicketSummary>
          rows={tickets}
          rowKey={(ticket) => ticket.id}
          columns={[
            { header: t('Nummer'), cell: (ticket) => `#${ticket.number}`, className: 'w-24' },
            {
              header: t('Onderwerp'),
              cell: (ticket) => (
                <Link to={ticketPath(scope, ticket.number)} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  {ticket.subject}
                </Link>
              ),
            },
            ...(staff ? [{ header: t('Klant'), cell: (ticket: TicketSummary) => ticket.customer_name }] : []),
            { header: t('Afdeling'), cell: (ticket) => ticket.department },
            { header: t('Status'), cell: (ticket) => <StatusBadge status={ticket.status} /> },
            { header: t('Laatste reactie'), cell: (ticket) => formatRelative(ticket.last_reply_at) },
          ]}
        />
      )}

      {data && tickets.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
      {!staff && <CreateTicketPanel open={creating} onClose={() => setCreating(false)} />}
    </>
  )
}
