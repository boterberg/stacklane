import { CalendarIcon, ChatBubbleLeftRightIcon, HashtagIcon, InboxIcon, PlusIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, PerPageSelect, SearchField, ToolbarSpacer, ViewToggle, type ListView } from '../../components/ui/ListToolbar'
import { DataTable, PaginationFooter, RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatRelative } from '../../lib/format'
import { defaultTicketStatuses, ticketStatusColors, ticketStatuses, ticketStatusLabels, type TicketStatus } from '../../lib/labels'
import { useTickets, type TicketSummary } from '../../lib/support'
import { useStoredState } from '../../lib/useStoredState'
import { CreateTicketPanel } from './CreateTicketPanel'

function StatusBadge({ status }: { status: TicketStatus }) {
  return <Badge color={ticketStatusColors[status]}>{ticketStatusLabels[status]}</Badge>
}

export function TicketsPage() {
  const [creating, setCreating] = useState(false)
  const [query, setQuery] = useState('')
  const [statuses, setStatuses] = useState<TicketStatus[]>(defaultTicketStatuses)
  const [page, setPage] = useState(1)
  const [view, setView] = useStoredState<ListView>('stacklane-view-tickets', 'grid')
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-tickets', 5)
  const { data, isPending } = useTickets({ q: query, statuses, page, perPage })

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
        title="Tickets"
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            Ticket maken
          </Button>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder="Zoek tickets..." />
        <ToolbarSpacer />
        <MultiSelectFilter
          values={statuses}
          options={ticketStatuses}
          labels={ticketStatusLabels}
          onChange={resetPage(setStatuses)}
          noun={['status', 'statussen']}
          placeholder="Filter op status..."
        />
        {tickets.length > 0 && <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />}
        <ViewToggle value={view} onChange={resetPage(setView)} />
      </ListToolbar>

      {!isPending && tickets.length === 0 ? (
        <EmptyState title="Geen tickets" description="Er zijn geen tickets gevonden." />
      ) : view === 'grid' ? (
        <RowCardList>
          {tickets.map((ticket) => (
            <RowCard
              key={ticket.id}
              to={`/support/tickets/${ticket.number}`}
              title={ticket.subject}
              badge={<StatusBadge status={ticket.status} />}
              meta={[
                { icon: HashtagIcon, label: ticket.number },
                { icon: InboxIcon, label: ticket.department },
                { icon: ChatBubbleLeftRightIcon, label: ticket.message_count === 1 ? '1 bericht' : `${ticket.message_count} berichten` },
                { icon: CalendarIcon, label: `Laatste reactie ${formatRelative(ticket.last_reply_at)}` },
              ]}
            />
          ))}
        </RowCardList>
      ) : (
        <DataTable<TicketSummary>
          rows={tickets}
          rowKey={(ticket) => ticket.id}
          columns={[
            { header: 'Nummer', cell: (ticket) => `#${ticket.number}`, className: 'w-24' },
            {
              header: 'Onderwerp',
              cell: (ticket) => (
                <Link to={`/support/tickets/${ticket.number}`} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
                  {ticket.subject}
                </Link>
              ),
            },
            { header: 'Afdeling', cell: (ticket) => ticket.department },
            { header: 'Status', cell: (ticket) => <StatusBadge status={ticket.status} /> },
            { header: 'Laatste reactie', cell: (ticket) => formatRelative(ticket.last_reply_at) },
          ]}
        />
      )}

      {data && tickets.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
      <CreateTicketPanel open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
