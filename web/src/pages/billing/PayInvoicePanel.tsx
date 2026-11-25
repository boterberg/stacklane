import { Radio, RadioGroup } from '@headlessui/react'
import { BuildingLibraryIcon, CreditCardIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { Checkbox } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { usePayInvoice, usePaymentOptions, type InvoiceDetail } from '../../lib/billing'
import { cn } from '../../lib/cn'
import { formatDate, formatMoney } from '../../lib/format'
import { t } from '../../lib/i18n'

type Provider = 'mollie' | 'stripe'

export function PayInvoicePanel({ invoice, open, onClose }: { invoice: InvoiceDetail; open: boolean; onClose: () => void }) {
  const { data: options } = usePaymentOptions()
  const pay = usePayInvoice(invoice.id)
  const available: Array<{ id: Provider; title: string; description: string }> = [
    ...(options?.providers.mollie ? [{ id: 'mollie' as const, title: t('iDEAL, Bancontact of kaart'), description: t('Je rondt de betaling af bij Mollie.') }] : []),
    ...(options?.providers.stripe ? [{ id: 'stripe' as const, title: t('Creditcard'), description: t('Je rondt de betaling af bij Stripe.') }] : []),
  ]
  const [chosen, setChosen] = useState<Provider | null>(null)
  const provider = chosen ?? available[0]?.id ?? null
  const [save, setSave] = useState(true)
  const renewable = invoice.lines.some((line) => line.subscription_code)
  const error = pay.error instanceof ApiError ? (pay.error.fieldErrors.provider ?? pay.error.message) : pay.error?.message

  return (
    <SlideOver
      open={open}
      onClose={onClose}
      title={t('Factuur #{number} betalen', { number: invoice.id })}
      description={t('Te betalen: {amount}', { amount: formatMoney(invoice.total_cents) })}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('Annuleren')}
          </Button>
          {provider && (
            <Button variant="primary" disabled={pay.isPending} onClick={() => pay.mutate({ provider, save: renewable && save })}>
              {t('Naar betalen')}
            </Button>
          )}
        </>
      }
    >
      <div className="space-y-6">
        {available.length > 0 && (
          <RadioGroup value={provider} onChange={setChosen} className="space-y-3" aria-label={t('Betaalmethode')}>
            {available.map((option) => (
              <Radio
                key={option.id}
                value={option.id}
                className={cn(
                  'flex cursor-pointer items-start gap-4 rounded-xl border bg-white px-5 py-4 focus:outline-none dark:bg-white/10',
                  provider === option.id ? 'border-zinc-800 dark:border-white' : 'border-zinc-200 dark:border-white/10',
                )}
              >
                <CreditCardIcon className="mt-0.5 size-5 text-zinc-400 dark:text-white/60" aria-hidden />
                <div>
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{option.title}</p>
                  <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{option.description}</p>
                </div>
              </Radio>
            ))}
          </RadioGroup>
        )}

        {available.length > 0 && renewable && (
          <Checkbox label={t('Gebruik deze betaalmethode ook voor automatische verlengingen')} checked={save} onChange={(event) => setSave(event.target.checked)} />
        )}

        {options?.providers.bank_transfer && options.iban && (
          <div className="flex gap-4 rounded-xl border border-zinc-200 bg-zinc-50 px-5 py-4 dark:border-white/10 dark:bg-white/5">
            <BuildingLibraryIcon className="mt-0.5 size-5 shrink-0 text-zinc-400 dark:text-white/60" aria-hidden />
            <div className="text-sm">
              <p className="font-medium text-zinc-800 dark:text-white">{t('Liever zelf overmaken?')}</p>
              <p className="mt-1 text-zinc-500 dark:text-white/70">
                {t('Maak {amount} over naar {iban} ten name van {holder}, onder vermelding van factuur {number}, voor {date}.', {
                  amount: formatMoney(invoice.total_cents),
                  iban: options.iban,
                  holder: options.account_holder ?? '',
                  number: invoice.id,
                  date: formatDate(invoice.due_at),
                })}
              </p>
            </div>
          </div>
        )}

        {options && available.length === 0 && !(options.providers.bank_transfer && options.iban) && (
          <p className="text-sm text-zinc-500 dark:text-white/70">{t('Online betalen is nog niet ingesteld. Neem contact op met support.')}</p>
        )}

        {error && <p className="text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
      </div>
    </SlideOver>
  )
}
