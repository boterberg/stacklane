import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'

import { links } from '../app/links'
import { ButtonLink } from '../components/ui/Button'
import { Card, CardText } from '../components/ui/Card'
import { PageHeader } from '../components/ui/Heading'
import { LinkCard } from '../components/ui/LinkCard'
import { api } from '../lib/api'
import { formatDate, formatRelative } from '../lib/format'
import { Badge } from '../components/ui/Badge'
import { ticketStatusColors, ticketStatusLabels, type TicketStatus } from '../lib/labels'

type Dashboard = {
  active_tickets: Array<{ id: string; number: number; subject: string; status: TicketStatus; department: string; last_reply_at: string }>
  expiring_soon: Array<{ kind: string; reference: string; name: string; expires_at: string }>
}

export function DashboardPage() {
  const { data } = useQuery({ queryKey: ['dashboard'], queryFn: () => api<Dashboard>('/dashboard') })

  return (
    <>
      <PageHeader title="Dashboard" />
      <div className="grid gap-6 xl:grid-cols-3">
        <div className="space-y-6 xl:col-span-2">
          <Card
            title="Actieve tickets"
            action={
              <ButtonLink to="/support/tickets" size="sm">
                Alle tickets
              </ButtonLink>
            }
          >
            {data && data.active_tickets.length > 0 ? (
              <ul className="-my-3 divide-y divide-zinc-200 dark:divide-white/10">
                {data.active_tickets.map((ticket) => (
                  <li key={ticket.id}>
                    <Link to={`/support/tickets/${ticket.number}`} className="flex items-center justify-between gap-4 py-3">
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{ticket.subject}</p>
                        <p className="text-sm text-zinc-500 dark:text-white/70">
                          #{ticket.number} · {ticket.department} · {formatRelative(ticket.last_reply_at)}
                        </p>
                      </div>
                      <Badge color={ticketStatusColors[ticket.status]}>{ticketStatusLabels[ticket.status]}</Badge>
                    </Link>
                  </li>
                ))}
              </ul>
            ) : (
              <CardText>Je hebt geen actieve tickets.</CardText>
            )}
          </Card>
          <Card title="Verloopt binnenkort" description="Abonnementen en domeinen die niet automatisch worden verlengd.">
            {data && data.expiring_soon.length > 0 ? (
              <ul className="-my-3 divide-y divide-zinc-200 dark:divide-white/10">
                {data.expiring_soon.map((service) => (
                  <li key={service.reference}>
                    <Link to={`/billing/subscriptions/${service.reference}`} className="flex items-center justify-between gap-4 py-3">
                      <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{service.name}</p>
                      <p className="shrink-0 text-sm text-zinc-500 dark:text-white/70">
                        {formatDate(service.expires_at)} · {formatRelative(service.expires_at)}
                      </p>
                    </Link>
                  </li>
                ))}
              </ul>
            ) : (
              <CardText>Er verloopt binnenkort niets.</CardText>
            )}
          </Card>
        </div>
        <div className="space-y-6">
          <LinkCard href={links.helpCenter} title="Helpcentrum" description="Bekijk ons uitgebreide helpcentrum voor stapsgewijze handleidingen en documentatie." />
          <LinkCard href={links.statusPage} title="Statuspagina" description="Bekijk actuele storingen en gepland onderhoud." />
        </div>
      </div>
    </>
  )
}
