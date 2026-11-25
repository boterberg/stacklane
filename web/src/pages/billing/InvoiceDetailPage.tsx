import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { ArrowDownTrayIcon, CalendarDaysIcon, CalendarIcon, CheckCircleIcon, ChevronDownIcon, ClockIcon, CreditCardIcon, PrinterIcon, XCircleIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useParams, useSearchParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { invoicePdfUrl, useInvoice, type InvoiceDetail } from '../../lib/billing'
import { formatDate, formatMoney } from '../../lib/format'
import { intlLocale, t } from '../../lib/i18n'
import { invoiceStatusColors, invoiceStatusLabels } from '../../lib/labels'
import { useMeta, type Company } from '../../lib/meta'
import { PayInvoicePanel } from './PayInvoicePanel'

const menuItem = 'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-zinc-800 data-focus:bg-zinc-50 dark:text-white dark:data-focus:bg-zinc-600'

function countryName(code?: string | null) {
  if (!code) return null
  return new Intl.DisplayNames([intlLocale()], { type: 'region' }).of(code) ?? code
}

export function DownloadButton({ href }: { href: string }) {
  return (
    <div className="flex">
      <a
        href={href}
        className="inline-flex h-10 items-center gap-2 rounded-l-lg border border-zinc-200 border-b-zinc-300/80 bg-white px-4 text-sm font-medium text-zinc-800 shadow-xs hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75"
      >
        <ArrowDownTrayIcon className="-ml-0.5 size-4" aria-hidden />
        {t('Factuur downloaden')}
      </a>
      <Menu>
        <MenuButton
          aria-label={t('Meer opties')}
          className="inline-flex h-10 items-center rounded-r-lg border border-l-0 border-zinc-200 border-b-zinc-300/80 bg-white px-2.5 text-zinc-800 shadow-xs hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75"
        >
          <ChevronDownIcon className="size-4" aria-hidden />
        </MenuButton>
        <MenuItems anchor={{ to: 'bottom end', gap: 4 }} className="z-50 min-w-44 rounded-lg border border-zinc-200 bg-white p-1 shadow-xs focus:outline-none dark:border-zinc-600 dark:bg-zinc-700">
          <MenuItem>
            <button type="button" className={menuItem} onClick={() => window.print()}>
              <PrinterIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
              {t('Afdrukken')}
            </button>
          </MenuItem>
        </MenuItems>
      </Menu>
    </div>
  )
}

export function InvoiceSpecification({ invoice, company }: { invoice: InvoiceDetail; company: Company | null }) {
  const customer = invoice.customer
  const taxRate = invoice.tax_rate_basis_points / 100
  return (
    <section className="max-w-4xl rounded-xl border border-zinc-200 bg-zinc-50 dark:border-white/7 dark:bg-white/5">
      <h2 className="px-6 py-4 text-sm font-medium text-zinc-800 dark:text-white">{t('Specificatie')}</h2>
      <div className="-mx-px rounded-xl border border-zinc-200 bg-white p-6 dark:border-white/7 dark:bg-white/4">
        <div className="flex flex-wrap justify-between gap-6 text-sm">
          <div className="text-zinc-500 dark:text-white/70">
            {customer.company_name && <p>{customer.company_name}</p>}
            <p>{customer.name}</p>
            {customer.address_line1 && <p>{customer.address_line1}</p>}
            {customer.address_line2 && <p>{customer.address_line2}</p>}
            {(customer.postal_code || customer.city) && <p>{[customer.postal_code, customer.city].filter(Boolean).join(' ')}</p>}
            {customer.country && <p>{countryName(customer.country)}</p>}
          </div>
          <div className="text-right text-zinc-500 dark:text-white/70">
            <p>{t('Factuurnummer: {number}', { number: invoice.id })}</p>
            <p>{t('Datum: {date}', { date: formatDate(invoice.issued_at) })}</p>
            <p>{t('Vervaldatum: {date}', { date: formatDate(invoice.due_at) })}</p>
          </div>
        </div>

        <table className="mt-8 w-full text-sm">
          <thead>
            <tr className="border-b border-zinc-200 dark:border-white/10">
              <th className="pb-3 text-left font-medium text-zinc-800 dark:text-white">{t('Beschrijving')}</th>
              <th className="pb-3 text-right font-medium text-zinc-800 dark:text-white">{t('Bedrag')}</th>
            </tr>
          </thead>
          <tbody>
            {invoice.lines.map((line, index) => (
              <tr key={index} className="border-b border-zinc-200 align-top dark:border-white/10">
                <td className="py-4">
                  <p className="text-zinc-800 dark:text-white">{line.description}</p>
                  {line.period_start && line.period_end && (
                    <p className="text-zinc-500 dark:text-white/70">{t('{start} t/m {end}', { start: formatDate(line.period_start), end: formatDate(line.period_end) })}</p>
                  )}
                  {line.subscription_code && <p className="text-zinc-500 dark:text-white/70">{t('Abonnement #{code}', { code: line.subscription_code })}</p>}
                </td>
                <td className="py-4 text-right text-zinc-800 dark:text-white">{formatMoney(line.amount_cents, invoice.currency)}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <dl className="mt-6 grid grid-cols-[1fr_auto] justify-items-end gap-x-10 gap-y-2 text-sm">
          <dt className="font-medium text-zinc-800 dark:text-white">{t('Totaal')}</dt>
          <dd className="font-medium text-zinc-800 dark:text-white">{formatMoney(invoice.total_cents, invoice.currency)}</dd>
          <dt className="text-zinc-500 dark:text-white/70">{t('Waarvan btw ({rate}%)', { rate: taxRate })}</dt>
          <dd className="text-zinc-500 dark:text-white/70">{formatMoney(invoice.tax_cents, invoice.currency)}</dd>
        </dl>
        {invoice.vat_scheme !== 'domestic' && (
          <p className="mt-6 text-sm text-zinc-500 dark:text-white/70">
            {invoice.vat_scheme === 'reverse_charge'
              ? t('Btw verlegd naar de afnemer (artikel 196 Btw-richtlijn). Btw-nummer afnemer: {number}.', { number: invoice.customer_vat_number ?? '-' })
              : invoice.vat_scheme === 'outside_eu'
                ? t('Dienst aan een afnemer buiten de EU, geen EU-btw in rekening gebracht.')
                : t('Btw tegen het tarief van {country} (OSS-regeling).', { country: invoice.vat_country ?? '-' })}
          </p>
        )}
      </div>
      {company?.name && (
        <div className="px-6 py-4 text-center text-xs text-zinc-500 dark:text-white/60">
          <p>{[company.name, company.coc_number && t('KVK {number}', { number: company.coc_number }), company.vat_number && t('BTW {number}', { number: company.vat_number })].filter(Boolean).join(' · ')}</p>
          <p>{[company.address, company.email, company.website].filter(Boolean).join(' · ')}</p>
        </div>
      )}
    </section>
  )
}

export function InvoiceDetailPage() {
  const { id = '' } = useParams()
  const [params] = useSearchParams()
  const returned = params.has('payment')
  const cancelled = params.get('cancelled') === '1'
  const { data: invoice, isPending, error } = useInvoice(id, returned && !cancelled)
  const { data: meta } = useMeta()
  const [paying, setPaying] = useState(false)

  if (isPending) return null
  if (error || !invoice) {
    return (
      <EmptyState
        title={t('Factuur niet gevonden')}
        description={t('Deze factuur bestaat niet of hoort niet bij je account.')}
        action={
          <ButtonLink to="/billing/invoices" size="sm">
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
              <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('Factuurdatum: {date}', { date: formatDate(invoice.issued_at) })}
            </span>
            <span className="flex items-center gap-2">
              <CalendarDaysIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
              {t('Vervaldatum: {date}', { date: formatDate(invoice.due_at) })}
            </span>
          </div>
        </div>
        <div className="flex gap-2">
          {invoice.status === 'open' && (
            <Button variant="primary" icon={CreditCardIcon} onClick={() => setPaying(true)}>
              {t('Betalen')}
            </Button>
          )}
          <DownloadButton href={invoicePdfUrl(invoice.id)} />
        </div>
      </div>

      {returned && <PaymentResult status={invoice.status} cancelled={cancelled} />}

      <InvoiceSpecification invoice={invoice} company={meta?.company ?? null} />
      {invoice.status === 'open' && <PayInvoicePanel invoice={invoice} open={paying} onClose={() => setPaying(false)} />}
    </>
  )
}

function PaymentResult({ status, cancelled }: { status: string; cancelled: boolean }) {
  const [Icon, text, tone] =
    status === 'paid'
      ? [CheckCircleIcon, t('Bedankt, je betaling is ontvangen.'), 'text-green-600 dark:text-green-400']
      : cancelled
        ? [XCircleIcon, t('Je hebt de betaling afgebroken. Je kunt het opnieuw proberen.'), 'text-zinc-400']
        : [ClockIcon, t('We verwerken je betaling. Dit duurt meestal een paar seconden.'), 'animate-pulse text-amber-500']
  return (
    <div className="mb-6 flex max-w-4xl items-center gap-3 rounded-xl border border-zinc-200 bg-white px-5 py-4 text-sm text-zinc-800 dark:border-white/10 dark:bg-white/10 dark:text-white">
      <Icon className={`size-5 shrink-0 ${tone}`} aria-hidden />
      {text}
    </div>
  )
}
