import { ArrowPathIcon, CalendarIcon, ClockIcon, CurrencyEuroIcon, HashtagIcon } from '@heroicons/react/24/outline'
import { useState, type ReactNode } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { Tabs } from '../../components/ui/Tabs'
import { expiryText, hasExpired, renewalText, useSubscription } from '../../lib/billing'
import { formatDate, formatEuro, formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { intervalLabels, invoiceStatusColors, invoiceStatusLabels, subscriptionStatusColors, subscriptionStatusLabels } from '../../lib/labels'

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="py-4 first:pt-0 last:pb-0">
      <dt className="text-sm font-medium text-zinc-800 dark:text-white">{label}</dt>
      <dd className="mt-1 text-sm text-zinc-500 dark:text-white/70">{children}</dd>
    </div>
  )
}

export function SubscriptionDetailPage() {
  const { code = '' } = useParams()
  const { data: subscription, isPending, error } = useSubscription(code)
  const [tab, setTab] = useState<'overview' | 'invoices'>('overview')

  if (isPending) return null
  if (error || !subscription) {
    return (
      <EmptyState
        title={t('Abonnement niet gevonden')}
        description={t('Dit abonnement bestaat niet of hoort niet bij je account.')}
        action={
          <ButtonLink to="/billing/subscriptions" size="sm">
            {t('Naar abonnementen')}
          </ButtonLink>
        }
      />
    )
  }

  const expired = hasExpired(subscription)
  const automatic = subscription.payment_mode === 'automatic'

  return (
    <>
      <div className="mb-6">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{subscription.name}</h1>
          <Badge color={subscriptionStatusColors[subscription.status]}>{subscriptionStatusLabels[subscription.status]}</Badge>
        </div>
        <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
          <span className="flex items-center gap-2">
            <ClockIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {expiryText(subscription)}
          </span>
          <span className="flex items-center gap-2">
            <CurrencyEuroIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {t('{price} per maand', { price: formatEuro(subscription.monthly_total_cents).replace('€', '').trim() })}
          </span>
          <span className="flex items-center gap-2">
            <ArrowPathIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {renewalText(subscription)}
          </span>
          <span className="flex items-center gap-2">
            <HashtagIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
            {subscription.code}
          </span>
        </div>
      </div>

      <Tabs
        tabs={[
          { id: 'overview', label: 'Overzicht' },
          { id: 'invoices', label: 'Facturen' },
        ]}
        active={tab}
        onChange={setTab}
      />

      {tab === 'overview' ? (
        <div className="grid max-w-5xl gap-6 lg:grid-cols-2 lg:items-start">
          <Card title={t('Verlenging en betaling')}>
            <dl className="divide-y divide-zinc-200 dark:divide-white/10">
              <Row label={expired ? t('Verlopen op') : t('Verloopt op')}>
                {formatDate(subscription.expires_at)} · {formatRelative(subscription.expires_at)}
              </Row>
              <Row label={t('Verlengingsinterval')}>{intervalLabels[subscription.billing_interval]}</Row>
              <Row label={t('Betaalmethode')}>
                {automatic ? t('Automatisch · We schrijven het bedrag af op de vervaldatum.') : t('Handmatig · Je krijgt een herinnering om te verlengen.')}
              </Row>
              <Row label={t('Begonnen op')}>{formatDate(subscription.started_at)}</Row>
            </dl>
          </Card>
          <Card title={t('Onderdelen')}>
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {subscription.items.map((item, index) => (
                <li key={index} className="flex items-start justify-between gap-4 pb-4">
                  <div>
                    <p className="text-sm font-medium text-zinc-800 dark:text-white">{item.description}</p>
                    {item.specs_line && <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{item.specs_line}</p>}
                  </div>
                  <p className="text-sm text-zinc-500 dark:text-white/70">{formatEuro(item.monthly_price_cents)}</p>
                </li>
              ))}
            </ul>
            <p className="flex justify-end gap-3 border-t border-zinc-200 pt-4 text-sm dark:border-white/10">
              <span className="text-zinc-500 dark:text-white/70">{t('Totaal per maand')}</span>
              <span className="font-medium text-zinc-800 dark:text-white">{formatEuro(subscription.monthly_total_cents)}</span>
            </p>
          </Card>
        </div>
      ) : subscription.invoices.length === 0 ? (
        <EmptyState title={t('Geen facturen')} description={t('Voor dit abonnement zijn nog geen facturen.')} />
      ) : (
        <RowCardList>
          {subscription.invoices.map((invoice) => (
            <RowCard
              key={invoice.id}
              to={`/billing/invoices/${invoice.id}`}
              title={`#${invoice.id}`}
              badge={<Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge>}
              meta={[
                { icon: CalendarIcon, label: formatDate(invoice.issued_at) },
                { icon: CurrencyEuroIcon, label: formatEuro(invoice.total_cents).replace('€', '').trim() },
              ]}
            />
          ))}
        </RowCardList>
      )}
    </>
  )
}
