import { CalendarIcon, HashtagIcon, InboxIcon, LockClosedIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { RichTextEditor } from '../../components/ui/RichTextEditor'
import { ApiError } from '../../lib/api'
import { cn } from '../../lib/cn'
import { formatDate, formatRelative } from '../../lib/format'
import { ticketStatusColors, ticketStatuses, ticketStatusLabels, type TicketStatus } from '../../lib/labels'
import { useCloseTicket, useReplyToTicket, useTicket, useUpdateTicketStatus, type TicketDetail, type TicketMessage, type TicketScope } from '../../lib/support'

function Message({ message }: { message: TicketMessage }) {
  return (
    <article className={cn('rounded-xl border', message.is_staff ? 'border-zinc-200 bg-zinc-50 dark:border-white/7 dark:bg-white/5' : 'border-zinc-200 bg-white dark:border-white/10 dark:bg-white/10')}>
      <header className="flex items-center justify-between gap-4 border-b border-zinc-200 px-6 py-3 dark:border-white/10">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-zinc-800 dark:text-white">{message.author_name}</span>
          {message.is_staff && <Badge color="blue">Support</Badge>}
        </div>
        <time dateTime={message.created_at} title={formatDate(message.created_at)} className="text-sm text-zinc-500 dark:text-white/60">
          {formatRelative(message.created_at)}
        </time>
      </header>
      <div className="rich-text px-6 py-4 text-sm text-zinc-700 dark:text-zinc-200" dangerouslySetInnerHTML={{ __html: message.body_html }} />
    </article>
  )
}

function ReplyForm({ ticket, scope }: { ticket: TicketDetail; scope: TicketScope }) {
  const [body, setBody] = useState('')
  const reply = useReplyToTicket(scope, ticket.number)
  const error = reply.error instanceof ApiError ? reply.error.fieldErrors.body_html : undefined

  function submit(event: FormEvent) {
    event.preventDefault()
    reply.mutate(body, { onSuccess: () => setBody('') })
  }

  return (
    <form onSubmit={submit} className="space-y-3">
      <h2 className="text-sm font-medium text-zinc-800 dark:text-white">
        {scope === 'staff' ? 'Reageren als support' : ticket.status === 'closed' ? 'Ticket heropenen met een reactie' : 'Reageren'}
      </h2>
      <RichTextEditor value={body} onChange={setBody} invalid={!!error} minHeight="min-h-40" />
      {error && <p className="text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
      <div className="flex justify-end">
        <Button type="submit" variant="primary" disabled={reply.isPending}>
          Versturen
        </Button>
      </div>
    </form>
  )
}

function StatusSelect({ ticket }: { ticket: TicketDetail }) {
  const update = useUpdateTicketStatus(ticket.number)
  return (
    <select
      aria-label="Status"
      value={ticket.status}
      disabled={update.isPending}
      onChange={(event) => update.mutate(event.target.value as TicketStatus)}
      className="h-8 rounded-md border border-zinc-200 border-b-zinc-300/80 bg-white px-3 text-sm text-zinc-700 shadow-xs dark:border-white/10 dark:bg-white/10 dark:text-zinc-200"
    >
      {ticketStatuses.map((status) => (
        <option key={status} value={status}>
          {ticketStatusLabels[status]}
        </option>
      ))}
    </select>
  )
}

export function TicketDetailPage({ scope = 'customer' }: { scope?: TicketScope }) {
  const { number = '' } = useParams()
  const { data: ticket, isPending, error } = useTicket(scope, number)
  const close = useCloseTicket(Number(number))
  const backTo = scope === 'staff' ? '/admin/tickets' : '/support/tickets'

  if (isPending) return null
  if (error || !ticket) {
    return (
      <EmptyState
        title="Ticket niet gevonden"
        description="Dit ticket bestaat niet of hoort niet bij je account."
        action={
          <ButtonLink to={backTo} size="sm">
            Naar tickets
          </ButtonLink>
        }
      />
    )
  }

  return (
    <>
      <div className="mb-8 flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{ticket.subject}</h1>
            <Badge color={ticketStatusColors[ticket.status]}>{ticketStatusLabels[ticket.status]}</Badge>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            <span className="flex items-center gap-2">
              <HashtagIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {ticket.number}
            </span>
            {scope === 'staff' && (
              <span className="flex items-center gap-2">
                <UserIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {ticket.customer_name} · {ticket.customer_email}
              </span>
            )}
            <span className="flex items-center gap-2">
              <InboxIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {ticket.department}
            </span>
            <span className="flex items-center gap-2">
              <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              Geopend op {formatDate(ticket.created_at)}
            </span>
          </div>
        </div>
        {scope === 'staff' ? (
          <StatusSelect ticket={ticket} />
        ) : (
          ticket.status !== 'closed' && (
            <Button size="sm" icon={LockClosedIcon} disabled={close.isPending} onClick={() => close.mutate(undefined)}>
              Ticket sluiten
            </Button>
          )
        )}
      </div>

      <div className="max-w-4xl space-y-4">
        {ticket.messages.map((message) => (
          <Message key={message.id} message={message} />
        ))}
        <div className="pt-4">
          <ReplyForm ticket={ticket} scope={scope} />
        </div>
      </div>
    </>
  )
}
