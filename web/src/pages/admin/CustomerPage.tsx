import { CalendarIcon, EnvelopeIcon, ShieldCheckIcon } from '@heroicons/react/24/outline'
import type { ReactNode } from 'react'
import { Link, useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { useCustomer } from '../../lib/admin'
import { countries } from '../../lib/countries'
import { formatDate, formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'
import {
  intervalLabels,
  invoiceStatusColors,
  invoiceStatusLabels,
  serviceKindLabels,
  serviceStatusColors,
  serviceStatusLabels,
  subscriptionStatusColors,
  subscriptionStatusLabels,
  ticketStatusColors,
  ticketStatusLabels,
} from '../../lib/labels'

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="py-4 first:pt-0 last:pb-0">
      <dt className="text-sm font-medium text-zinc-800 dark:text-white">{label}</dt>
      <dd className="mt-1 text-sm break-words text-zinc-500 dark:text-white/70">{children}</dd>
    </div>
  )
}

function Line({ title, to, detail, badge }: { title: string; to?: string; detail: string; badge: ReactNode }) {
  return (
    <li className="flex items-start justify-between gap-4 py-3 first:pt-0 last:pb-0">
      <div className="min-w-0">
        {to ? (
          <Link to={to} className="text-sm font-medium text-zinc-800 hover:underline dark:text-white">
            {title}
          </Link>
        ) : (
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{title}</p>
        )}
        <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{detail}</p>
      </div>
      <div className="shrink-0">{badge}</div>
    </li>
  )
}

function Empty({ children }: { children: string }) {
  return <p className="text-sm text-zinc-500 dark:text-white/70">{children}</p>
}

export function CustomerPage() {
  const { id = '' } = useParams()
  const { data: customer, isPending, error } = useCustomer(id)

  if (isPending) return null
  if (error || !customer) {
    return (
      <EmptyState
        title={t('Klant niet gevonden')}
        description={t('Deze klant bestaat niet.')}
        action={
          <ButtonLink to="/admin/customers" size="sm">
            {t('Naar klanten')}
          </ButtonLink>
        }
      />
    )
  }

  const { profile } = customer
  const name = `${profile.first_name} ${profile.last_name}`.trim() || profile.email
  const country = countries().find((option) => option.code === profile.country)?.name ?? profile.country
  const address = [profile.address_line1, profile.address_line2, [profile.postal_code, profile.city].filter(Boolean).join(' '), country].filter(Boolean)

  return (
    <>
      <div className="mb-8">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{name}</h1>
          {profile.company_name && <Badge color="zinc">{profile.company_name}</Badge>}
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
          <span className="flex items-center gap-2">
            <EnvelopeIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {profile.email}
          </span>
          <span className="flex items-center gap-2">
            <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {t('Klant sinds {date}', { date: formatDate(profile.created_at) })}
          </span>
          <span className="flex items-center gap-2">
            <ShieldCheckIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {profile.two_factor ? t('Tweestapsverificatie aan') : t('Tweestapsverificatie uit')}
          </span>
        </div>
      </div>

      <div className="grid max-w-6xl gap-6 lg:grid-cols-2 lg:items-start">
        <Card title={t('Gegevens')}>
          <dl className="divide-y divide-zinc-200 dark:divide-white/10">
            <Row label={t('Adres')}>{address.length > 0 ? address.join(', ') : '-'}</Row>
            <Row label={t('Telefoonnummer')}>{profile.phone ?? '-'}</Row>
            {profile.is_business && <Row label={t('KVK-nummer')}>{profile.coc_number ?? '-'}</Row>}
            {profile.is_business && <Row label={t('Btw-nummer')}>{profile.tax_id ?? '-'}</Row>}
            <Row label={t('E-mailadres bevestigd')}>{profile.email_verified_at ? formatDate(profile.email_verified_at) : t('Nog niet')}</Row>
          </dl>
        </Card>

        <Card title={t('Abonnementen')}>
          {customer.subscriptions.length === 0 ? (
            <Empty>{t('Deze klant heeft geen abonnementen.')}</Empty>
          ) : (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {customer.subscriptions.map((subscription) => (
                <Line
                  key={subscription.id}
                  title={`${subscription.code} · ${subscription.description}`}
                  detail={`${formatEuro(subscription.monthly_cents)} ${t('per maand')} · ${intervalLabels[subscription.billing_interval]} · ${t('Verloopt op')} ${formatDate(subscription.expires_at)}`}
                  badge={<Badge color={subscriptionStatusColors[subscription.status]}>{subscriptionStatusLabels[subscription.status]}</Badge>}
                />
              ))}
            </ul>
          )}
        </Card>

        <Card title={t('Diensten')}>
          {customer.services.length === 0 ? (
            <Empty>{t('Deze klant heeft geen diensten.')}</Empty>
          ) : (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {customer.services.map((service) => (
                <Line
                  key={service.id}
                  title={service.name}
                  detail={`${serviceKindLabels[service.kind] ?? service.kind} · ${formatDate(service.created_at)}`}
                  badge={<Badge color={serviceStatusColors[service.status]}>{serviceStatusLabels[service.status]}</Badge>}
                />
              ))}
            </ul>
          )}
        </Card>

        <Card title={t('Laatste facturen')}>
          {customer.invoices.length === 0 ? (
            <Empty>{t('Deze klant heeft nog geen facturen.')}</Empty>
          ) : (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {customer.invoices.map((invoice) => (
                <Line
                  key={invoice.id}
                  title={`#${invoice.id}`}
                  to={`/admin/invoices/${invoice.id}`}
                  detail={`${formatEuro(invoice.total_cents)} · ${formatDate(invoice.issued_at)}`}
                  badge={<Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge>}
                />
              ))}
            </ul>
          )}
        </Card>

        <Card title={t('Laatste tickets')}>
          {customer.tickets.length === 0 ? (
            <Empty>{t('Deze klant heeft nog geen tickets.')}</Empty>
          ) : (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {customer.tickets.map((ticket) => (
                <Line
                  key={ticket.number}
                  title={`#${ticket.number} · ${ticket.subject}`}
                  to={`/admin/tickets/${ticket.number}`}
                  detail={formatDate(ticket.last_reply_at)}
                  badge={<Badge color={ticketStatusColors[ticket.status]}>{ticketStatusLabels[ticket.status]}</Badge>}
                />
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  )
}
