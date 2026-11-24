import { Radio, RadioGroup } from '@headlessui/react'
import { BellIcon, CreditCardIcon, InformationCircleIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Field, Select } from '../../components/ui/Field'
import { usePaymentDefaults, useRevokeMandate, useSavePaymentDefaults, type PaymentDefaults } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { usePaymentOptions, type Mandate } from '../../lib/billing'
import { cn } from '../../lib/cn'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'

const modes = [
  {
    value: 'manual' as const,
    icon: BellIcon,
    title: 'Handmatig',
    text: 'Je krijgt een herinnering als een abonnement bijna verloopt en betaalt de factuur zelf online.',
  },
  {
    value: 'automatic' as const,
    icon: CreditCardIcon,
    title: 'Automatisch',
    text: 'Verlengingen worden afgeschreven van een opgeslagen betaalmethode.',
  },
]

function DefaultMode({ initial, mandates }: { initial: PaymentDefaults; mandates: Mandate[] }) {
  const [form, setForm] = useState(initial)
  const save = useSavePaymentDefaults()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const changed = form.mode !== initial.mode || form.mandate_id !== initial.mandate_id

  return (
    <Card
      title={t('Standaard betaalwijze')}
      description={t('Hoe je abonnementen verlengd worden. Per abonnement kun je dit daarna nog aanpassen.')}
      footer={
        <>
          {save.isSuccess && !changed && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span>}
          <Button variant="primary" disabled={!changed || save.isPending} onClick={() => save.mutate(form)}>
            {t('Voorkeuren opslaan')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        <RadioGroup
          value={form.mode}
          onChange={(mode) => setForm({ mode, mandate_id: mode === 'automatic' ? (form.mandate_id ?? mandates[0]?.id ?? null) : null })}
          className="space-y-3"
          aria-label={t('Standaard betaalwijze')}
        >
          {modes.map((mode) => {
            const disabled = mode.value === 'automatic' && mandates.length === 0
            return (
              <Radio
                key={mode.value}
                value={mode.value}
                disabled={disabled}
                className={cn(
                  'flex cursor-pointer items-start gap-3 rounded-xl border px-4 py-4 focus:outline-none data-disabled:cursor-not-allowed data-disabled:opacity-50',
                  form.mode === mode.value ? 'border-zinc-800 dark:border-white' : 'border-zinc-200 dark:border-white/10',
                )}
              >
                <mode.icon className="mt-0.5 size-4.5 shrink-0 text-zinc-500 dark:text-white/60" aria-hidden />
                <div className="flex-1">
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{t(mode.title)}</p>
                  <p className="mt-1 text-xs text-zinc-500 dark:text-white/70">{t(mode.text)}</p>
                </div>
                <span
                  className={cn(
                    'mt-0.5 size-4.5 shrink-0 rounded-full border',
                    form.mode === mode.value ? 'border-[5px] border-zinc-800 dark:border-white' : 'border-zinc-300 dark:border-white/20',
                  )}
                  aria-hidden
                />
              </Radio>
            )
          })}
        </RadioGroup>
        {form.mode === 'automatic' && mandates.length > 0 && (
          <Field label={t('Betaalmethode')} error={errors.mandate_id}>
            {(id) => (
              <Select id={id} value={form.mandate_id ?? ''} onChange={(event) => setForm({ ...form, mandate_id: event.target.value })}>
                {mandates.map((mandate) => (
                  <option key={mandate.id} value={mandate.id}>
                    {mandate.description}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        )}
      </div>
    </Card>
  )
}

export function PaymentMethodsTab() {
  const { data: options } = usePaymentOptions()
  const { data: defaults } = usePaymentDefaults()
  const revoke = useRevokeMandate()
  const [removing, setRemoving] = useState<Mandate | null>(null)
  const mandates = options?.mandates ?? []

  return (
    <div className="max-w-2xl space-y-6">
      <Card title={t('Opgeslagen betaalmethodes')} description={t('Betaalmethodes die je voor automatische verlengingen hebt bewaard.')}>
        {options && mandates.length === 0 ? (
          <div className="flex gap-3 rounded-lg border border-zinc-200 bg-zinc-50 px-4 py-4 text-sm text-zinc-600 dark:border-white/10 dark:bg-white/5 dark:text-white/70">
            <InformationCircleIcon className="mt-0.5 size-5 shrink-0 text-zinc-400" aria-hidden />
            {t('Je hebt nog geen betaalmethode bewaard. Betaal je een factuur online, vink dan aan dat je de betaalmethode wilt gebruiken voor automatische verlengingen.')}
          </div>
        ) : (
          <ul className="divide-y divide-zinc-200 dark:divide-white/10">
            {mandates.map((mandate) => (
              <li key={mandate.id} className="flex items-center justify-between gap-4 py-3 first:pt-0 last:pb-0">
                <div className="flex items-center gap-3">
                  <CreditCardIcon className="size-5 text-zinc-400 dark:text-white/50" aria-hidden />
                  <div className="text-sm">
                    <p className="font-medium text-zinc-800 dark:text-white">{mandate.description}</p>
                    <p className="text-zinc-500 dark:text-white/60">
                      {mandate.provider === 'mollie' ? 'Mollie' : 'Stripe'} · {t('bewaard op {date}', { date: formatDate(mandate.created_at) })}
                    </p>
                  </div>
                </div>
                <Button size="sm" icon={TrashIcon} onClick={() => setRemoving(mandate)}>
                  {t('Verwijderen')}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>
      {defaults && options && <DefaultMode key={`${defaults.mode}-${defaults.mandate_id}-${mandates.length}`} initial={defaults} mandates={mandates} />}
      <ConfirmDialog
        open={removing !== null}
        title={t('Betaalmethode verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={revoke.isPending}
        onClose={() => setRemoving(null)}
        onConfirm={() => removing && revoke.mutate(removing.id, { onSettled: () => setRemoving(null) })}
      >
        {t('Abonnementen die automatisch met {method} verlengen, gaan terug naar handmatig betalen.', { method: removing?.description ?? '' })}
      </ConfirmDialog>
    </div>
  )
}
