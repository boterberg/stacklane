import { useState, type FormEvent, type ReactNode } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { usePaymentSettings, useSavePaymentSettings, useSaveSettings, useSettings, type BillingSettings, type Company, type PaymentSettings } from '../../lib/admin'
import { countries } from '../../lib/countries'
import { t } from '../../lib/i18n'

function Saved({ show }: { show: boolean }) {
  return show ? <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span> : null
}

function FormCard({ title, description, onSubmit, pending, saved, children }: { title: string; description: string; onSubmit: () => void; pending: boolean; saved: boolean; children: ReactNode }) {
  function submit(event: FormEvent) {
    event.preventDefault()
    onSubmit()
  }
  return (
    <Card title={title} description={description}>
      <form onSubmit={submit} className="space-y-6" noValidate>
        {children}
        <div className="flex items-center justify-end gap-3">
          <Saved show={saved} />
          <Button type="submit" variant="primary" disabled={pending}>
            {t('Opslaan')}
          </Button>
        </div>
      </form>
    </Card>
  )
}

function CompanyForm({ initial }: { initial: Company }) {
  const [company, setCompany] = useState(initial)
  const save = useSaveSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const field = (key: keyof Company) => ({
    value: company[key] ?? '',
    onChange: (event: { target: { value: string } }) => setCompany({ ...company, [key]: event.target.value }),
  })
  return (
    <FormCard title={t('Bedrijf')} description={t('Deze gegevens staan op elke factuur.')} onSubmit={() => save.mutate({ company })} pending={save.isPending} saved={save.isSuccess}>
      <Field label={t('Bedrijfsnaam')} error={errors.name}>
        {(id) => <Input id={id} invalid={!!errors.name} {...field('name')} />}
      </Field>
      <Field label={t('Adres')} description={t('Op één regel, bijvoorbeeld Kerkstraat 1, 1234 AB Amsterdam.')}>
        {(id) => <Input id={id} {...field('address')} />}
      </Field>
      <div className="grid gap-6 sm:grid-cols-2">
        <Field label={t('E-mailadres')}>{(id) => <Input id={id} type="email" {...field('email')} />}</Field>
        <Field label={t('Website')}>{(id) => <Input id={id} {...field('website')} />}</Field>
        <Field label={t('KVK-nummer')}>{(id) => <Input id={id} {...field('coc_number')} />}</Field>
        <Field label={t('Btw-nummer')}>{(id) => <Input id={id} {...field('vat_number')} />}</Field>
        <Field label={t('IBAN')}>{(id) => <Input id={id} {...field('iban')} />}</Field>
        <Field label={t('BIC')}>{(id) => <Input id={id} {...field('bic')} />}</Field>
      </div>
    </FormCard>
  )
}

function BillingForm({ initial }: { initial: BillingSettings }) {
  const [billing, setBilling] = useState(initial)
  const save = useSaveSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const number = (key: 'tax_rate_basis_points' | 'payment_term_days' | 'renewal_notice_days' | 'suspend_after_days', scale = 1) => ({
    inputMode: 'numeric' as const,
    value: String(billing[key] / scale),
    onChange: (event: { target: { value: string } }) => setBilling({ ...billing, [key]: Math.round(Number(event.target.value.replace(',', '.')) * scale) || 0 }),
  })
  return (
    <FormCard
      title={t('Facturatie')}
      description={t('Wanneer Stacklane factureert en wat er gebeurt als een klant niet betaalt.')}
      onSubmit={() => save.mutate({ billing })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <div className="grid gap-6 sm:grid-cols-2">
        <Field label={t('Land van je bedrijf')} error={errors.home_country}>
          {(id) => (
            <Select id={id} value={billing.home_country} onChange={(event) => setBilling({ ...billing, home_country: event.target.value })}>
              {countries().map((country) => (
                <option key={country.code} value={country.code}>
                  {country.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('Btw-tarief (%)')} description={t('Je prijzen zijn inclusief dit tarief.')} error={errors.tax_rate_basis_points}>
          {(id) => <Input id={id} {...number('tax_rate_basis_points', 100)} />}
        </Field>
        <div className="sm:col-span-2">
          <Checkbox label={t('OSS-regeling gebruiken')} checked={billing.oss} onChange={(event) => setBilling({ ...billing, oss: event.target.checked })} />
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">
            {t('Particulieren in een ander EU-land betalen dan het btw-tarief van hun eigen land. Zakelijke klanten met een geldig EU-btw-nummer krijgen altijd verlegde btw.')}
          </p>
        </div>
        <Field label={t('Betaaltermijn (dagen)')}>{(id) => <Input id={id} {...number('payment_term_days')} />}</Field>
        <Field label={t('Factuur maken (dagen voor verlengen)')}>{(id) => <Input id={id} {...number('renewal_notice_days')} />}</Field>
        <Field label={t('Opschorten na (dagen te laat)')} error={errors.billing}>
          {(id) => <Input id={id} {...number('suspend_after_days')} />}
        </Field>
      </div>
    </FormCard>
  )
}

function PaymentsForm({ initial }: { initial: PaymentSettings }) {
  const [form, setForm] = useState({
    mollie_enabled: initial.mollie_enabled,
    stripe_enabled: initial.stripe_enabled,
    bank_transfer_enabled: initial.bank_transfer_enabled,
    mollie_key: '',
    stripe_secret_key: '',
    stripe_webhook_secret: '',
  })
  const save = useSavePaymentSettings()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const stored = t('Opgeslagen. Laat leeg om te houden.')
  return (
    <FormCard
      title={t('Betalingen')}
      description={t('Hoe klanten hun facturen betalen. Sleutels worden versleuteld opgeslagen.')}
      onSubmit={() => save.mutate(form, { onSuccess: () => setForm((current) => ({ ...current, mollie_key: '', stripe_secret_key: '', stripe_webhook_secret: '' })) })}
      pending={save.isPending}
      saved={save.isSuccess}
    >
      <div className="space-y-4">
        <Checkbox
          label={t('Mollie: iDEAL, Bancontact, kaart en automatisch incasseren')}
          checked={form.mollie_enabled}
          onChange={(event) => setForm({ ...form, mollie_enabled: event.target.checked })}
        />
        <Field label={t('API-sleutel van Mollie')} description={t('Begint met live_ of test_. De webhook stelt Stacklane zelf in.')} error={errors.mollie_key}>
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.mollie_configured ? stored : 'live_...'}
              invalid={!!errors.mollie_key}
              value={form.mollie_key}
              onChange={(event) => setForm({ ...form, mollie_key: event.target.value })}
            />
          )}
        </Field>
      </div>
      <div className="space-y-4 border-t border-zinc-200 pt-6 dark:border-white/10">
        <Checkbox label={t('Stripe: creditcards en opgeslagen kaarten')} checked={form.stripe_enabled} onChange={(event) => setForm({ ...form, stripe_enabled: event.target.checked })} />
        <Field label={t('Geheime sleutel van Stripe')} error={errors.stripe_secret_key}>
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.stripe_configured ? stored : 'sk_live_...'}
              invalid={!!errors.stripe_secret_key}
              value={form.stripe_secret_key}
              onChange={(event) => setForm({ ...form, stripe_secret_key: event.target.value })}
            />
          )}
        </Field>
        <Field
          label={t('Webhookgeheim van Stripe')}
          description={
            <>
              {t('Maak in Stripe een webhook naar dit adres, met de events checkout.session.completed, checkout.session.expired, payment_intent.succeeded en payment_intent.payment_failed:')}{' '}
              <code className="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-800 dark:bg-white/10 dark:text-white">{initial.webhook_urls.stripe}</code>
            </>
          }
        >
          {(id) => (
            <Input
              id={id}
              type="password"
              autoComplete="off"
              placeholder={initial.stripe_webhook_configured ? stored : 'whsec_...'}
              value={form.stripe_webhook_secret}
              onChange={(event) => setForm({ ...form, stripe_webhook_secret: event.target.value })}
            />
          )}
        </Field>
      </div>
      <div className="border-t border-zinc-200 pt-6 dark:border-white/10">
        <Checkbox
          label={t('Overboeking: toon je IBAN op facturen en bij het betalen')}
          checked={form.bank_transfer_enabled}
          onChange={(event) => setForm({ ...form, bank_transfer_enabled: event.target.checked })}
        />
      </div>
    </FormCard>
  )
}

export function SettingsPage() {
  const { data: settings } = useSettings()
  const { data: payments } = usePaymentSettings()
  return (
    <>
      <PageHeader title={t('Instellingen')} />
      <div className="max-w-3xl space-y-6">
        {settings && <CompanyForm initial={settings.company} />}
        {settings && <BillingForm initial={settings.billing} />}
        {payments && <PaymentsForm initial={payments} />}
      </div>
    </>
  )
}
