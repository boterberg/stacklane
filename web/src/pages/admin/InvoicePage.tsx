import { CalendarDaysIcon, CalendarIcon, CheckIcon, UserIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { ApiError } from '../../lib/api'
import { useAdminInvoice, useInvoiceAction } from '../../lib/admin'
import { formatDate, formatEuro, formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { invoiceStatusColors, invoiceStatusLabels } from '../../lib/labels'
import { useMeta } from '../../lib/meta'
import { DownloadButton, InvoiceSpecification } from '../billing/InvoiceDetailPage'

const providerLabels: Record<string, string> = {
  mollie: 'Mollie',
  stripe: 'Stripe',
  manual: 'Handmatig',
  bank_transfer: 'Overboeking',
}

export function AdminInvoicePage() {
  const { id = '' } = useParams()
  const { data: invoice, isPending, error } = useAdminInvoice(id)
  const { data: meta } = useMeta()
  const action = useInvoiceAction(Number(id))
  const [confirm, setConfirm] = useState<'mark-paid' | 'cancel' | null>(null)
  const actionError = action.error instanceof ApiError ? (action.error.fieldErrors.invoice ?? action.error.message) : null

  if (isPending) return null
  if (error || !invoice) {
    return (
      <EmptyState
        title={t('Factuur niet gevonden')}
        description={t('Deze factuur bestaat niet.')}
        action={
          <ButtonLink to="/admin/invoices" size="sm">
            {t('Naar facturen')}
          </ButtonLink>
        }
      />
    )
  }

  return (
    <>
      <div className="mb-8 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{t('Factuur #{number}', { number: invoice.id })}</h1>
            <Badge color={invoiceStatusColors[invoice.status]}>{invoiceStatusLabels[invoice.status]}</Badge>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            <span className="flex items-center gap-2">
              <UserIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {invoice.customer.name} · {invoice.customer_email}
            </span>
            <span className="flex items-center gap-2">
              <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('Factuurdatum: {date}', { date: formatDate(invoice.issued_at) })}
            </span>
            <span className="flex items-center gap-2">
              <CalendarDaysIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('Vervaldatum: {date}', { date: formatDate(invoice.due_at) })}
            </span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          {invoice.status === 'open' && (
            <>
              <Button icon={XMarkIcon} onClick={() => setConfirm('cancel')}>
                {t('Annuleren')}
              </Button>
              <Button variant="primary" icon={CheckIcon} onClick={() => setConfirm('mark-paid')}>
                {t('Op betaald zetten')}
              </Button>
            </>
          )}
          <DownloadButton href={`/api/admin/invoices/${invoice.id}/pdf`} />
        </div>
      </div>
      {actionError && <p className="mb-6 text-sm font-medium text-red-500 dark:text-red-400">{actionError}</p>}

      <div className="max-w-4xl space-y-6">
        <InvoiceSpecification invoice={invoice} company={meta?.company ?? null} />
        <Card title={t('Betalingen')}>
          {invoice.payments.length === 0 ? (
            <p className="text-sm text-zinc-500 dark:text-white/70">{t('Er is nog niet geprobeerd te betalen.')}</p>
          ) : (
            <ul className="-my-3 divide-y divide-zinc-200 dark:divide-white/10">
              {invoice.payments.map((payment, index) => (
                <li key={index} className="flex flex-wrap items-center justify-between gap-4 py-3 text-sm">
                  <div>
                    <p className="font-medium text-zinc-800 dark:text-white">
                      {t(providerLabels[payment.provider] ?? payment.provider)}
                      {payment.method && payment.method !== 'manual' && <span className="font-normal text-zinc-500 dark:text-white/60"> · {payment.method}</span>}
                    </p>
                    <p className="text-zinc-500 dark:text-white/70">{formatRelative(payment.created_at)}</p>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-zinc-800 dark:text-white">{formatEuro(payment.amount_cents)}</span>
                    <Badge color={payment.status === 'paid' ? 'green' : payment.status === 'open' ? 'amber' : 'zinc'}>{t(paymentStatusLabels[payment.status] ?? payment.status)}</Badge>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>

      <ConfirmDialog
        open={confirm === 'mark-paid'}
        title={t('Factuur op betaald zetten?')}
        confirmLabel={t('Op betaald zetten')}
        pending={action.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => action.mutate('mark-paid', { onSettled: () => setConfirm(null) })}
      >
        {t('Doe dit als {amount} op je rekening staat. Abonnementen op deze factuur worden dan verlengd en opgeschorte diensten gaan weer aan.', {
          amount: formatEuro(invoice.total_cents),
        })}
      </ConfirmDialog>
      <ConfirmDialog
        open={confirm === 'cancel'}
        title={t('Factuur annuleren?')}
        confirmLabel={t('Factuur annuleren')}
        danger
        pending={action.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => action.mutate('cancel', { onSettled: () => setConfirm(null) })}
      >
        {t('De klant hoeft deze factuur dan niet meer te betalen. Dit kun je niet ongedaan maken.')}
      </ConfirmDialog>
    </>
  )
}

const paymentStatusLabels: Record<string, string> = {
  open: 'Bezig',
  paid: 'Betaald',
  failed: 'Mislukt',
  cancelled: 'Afgebroken',
  expired: 'Verlopen',
}
