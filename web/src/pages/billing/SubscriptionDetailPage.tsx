import { ArrowPathIcon, ArrowUturnLeftIcon, CalendarIcon, ClockIcon, CurrencyEuroIcon, HashtagIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useState, type ReactNode } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { Tabs } from '../../components/ui/Tabs'
import { expiryText, hasExpired, renewalText, useCancelSubscription, usePaymentOptions, useResumeSubscription, useSetPaymentMethod, useSubscription, type SubscriptionDetail } from '../../lib/billing'
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

function PaymentMethod({ subscription }: { subscription: SubscriptionDetail }) {
  const { data: options } = usePaymentOptions()
  const change = useSetPaymentMethod(subscription.code)
  const mandates = options?.mandates ?? []
  const description = subscription.mandate
    ? t('Automatisch met {method} · We schrijven het bedrag af op de vervaldatum.', { method: subscription.mandate })
    : t('Handmatig · Je krijgt een herinnering om te verlengen.')
  if (mandates.length === 0 || subscription.cancelled_at) return <>{description}</>
  return (
    <div className="space-y-3">
      <p>{description}</p>
      <select
        aria-label={t('Betaalmethode')}
        value={subscription.mandate_id ?? ''}
        disabled={change.isPending}
        onChange={(event) => change.mutate(event.target.value || null)}
        className="h-8 rounded-md border border-zinc-200 border-b-zinc-300/80 bg-white px-3 text-sm text-zinc-700 shadow-xs dark:border-white/10 dark:bg-white/10 dark:text-zinc-200"
      >
        <option value="">{t('Handmatig betalen')}</option>
        {mandates.map((mandate) => (
          <option key={mandate.id} value={mandate.id}>
            {t('Automatisch met {method}', { method: mandate.description })}
          </option>
        ))}
      </select>
    </div>
  )
}

function SubscriptionActions({ subscription }: { subscription: SubscriptionDetail }) {
  const [confirming, setConfirming] = useState(false)
  const cancel = useCancelSubscription(subscription.code)
  const resume = useResumeSubscription(subscription.code)
  const running = ['active', 'pending', 'suspended'].includes(subscription.status)
  if (!running || hasExpired(subscription)) return null
  if (subscription.cancelled_at) {
    return (
      <Button size="sm" icon={ArrowUturnLeftIcon} disabled={resume.isPending} onClick={() => resume.mutate(undefined)}>
        {t('Opzegging intrekken')}
      </Button>
    )
  }
  return (
    <>
      <Button size="sm" icon={XMarkIcon} onClick={() => setConfirming(true)}>
        {t('Opzeggen')}
      </Button>
      <ConfirmDialog
        open={confirming}
        title={t('Abonnement opzeggen?')}
        confirmLabel={t('Opzeggen')}
        danger
        pending={cancel.isPending}
        onClose={() => setConfirming(false)}
        onConfirm={() => cancel.mutate(undefined, { onSettled: () => setConfirming(false) })}
      >
        {t('Je abonnement loopt door tot {date} en wordt daarna niet verlengd. Tot die dag kun je de opzegging nog intrekken.', {
          date: formatDate(subscription.expires_at),
        })}
      </ConfirmDialog>
    </>
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

  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
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
        <SubscriptionActions subscription={subscription} />
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
                <PaymentMethod subscription={subscription} />
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
