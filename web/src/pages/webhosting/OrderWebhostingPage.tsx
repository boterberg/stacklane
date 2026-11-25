import { Radio, RadioGroup } from '@headlessui/react'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { cn } from '../../lib/cn'
import { formatEuro } from '../../lib/format'
import { t } from '../../lib/i18n'
import { intervalLabels } from '../../lib/labels'
import { usePlaceOrder, useShopProducts } from '../../lib/vps'

const intervals = [
  { value: 'monthly', months: 1 },
  { value: 'quarterly', months: 3 },
  { value: 'half_yearly', months: 6 },
  { value: 'yearly', months: 12 },
]

export function OrderWebhostingPage() {
  const { data: products, isPending } = useShopProducts('webhosting')
  const place = usePlaceOrder()
  const navigate = useNavigate()
  const [productId, setProductId] = useState<string | null>(null)
  const [interval, setInterval] = useState('monthly')
  const [name, setName] = useState('')
  const errors = place.error instanceof ApiError ? place.error.fieldErrors : {}

  const product = (products ?? []).find((candidate) => candidate.id === productId) ?? products?.[0]
  const months = intervals.find((option) => option.value === interval)?.months ?? 1

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!product) return
    place.mutate(
      { product_id: product.id, billing_interval: interval, name, image: null, ssh_key_ids: [] },
      { onSuccess: (order) => navigate(`/billing/invoices/${order.invoice_id}`) },
    )
  }

  if (!isPending && (products ?? []).length === 0) {
    return (
      <>
        <PageHeader title={t('Webhosting bestellen')} />
        <EmptyState title={t('Geen pakketten')} description={t('Er zijn nog geen webhostingpakketten te bestellen.')} />
      </>
    )
  }

  return (
    <>
      <PageHeader title={t('Webhosting bestellen')} description={t('Kies een pakket. Na betaling staat je webhosting binnen een paar minuten klaar.')} />
      <form onSubmit={submit} noValidate className="grid grid-cols-1 max-w-5xl gap-6 lg:grid-cols-[1fr_20rem]">
        <div className="space-y-6">
          <Card title={t('Pakket')}>
            <RadioGroup value={product?.id ?? null} onChange={setProductId} className="grid grid-cols-1 gap-3 sm:grid-cols-2" aria-label={t('Pakket')}>
              {(products ?? []).map((option) => (
                <Radio
                  key={option.id}
                  value={option.id}
                  className={cn(
                    'cursor-pointer rounded-xl border px-4 py-4 focus:outline-none',
                    product?.id === option.id ? 'border-zinc-800 dark:border-white' : 'border-zinc-200 dark:border-white/10',
                  )}
                >
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{option.name}</p>
                  {option.description && <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{option.description}</p>}
                  <p className="mt-3 text-sm font-medium text-zinc-800 dark:text-white">{t('{price} per maand', { price: formatEuro(option.monthly_price_cents) })}</p>
                </Radio>
              ))}
            </RadioGroup>
          </Card>
          <Card title={t('Website')}>
            <Field label={t('Domeinnaam')} description={t('Het domein van je website, bijvoorbeeld jouwbedrijf.nl. Heb je er nog geen?')} error={errors.name}>
              {(id) => <Input id={id} invalid={!!errors.name} placeholder="jouwbedrijf.nl" value={name} onChange={(event) => setName(event.target.value)} />}
            </Field>
            <p className="mt-3 text-sm">
              <Link to="/domains/register" className="font-medium text-zinc-800 underline-offset-4 hover:underline dark:text-white">
                {t('Registreer eerst een domein')}
              </Link>
            </p>
          </Card>
        </div>
        <div>
          <Card title={t('Overzicht')}>
            <div className="space-y-6">
              <Field label={t('Betalen per')} error={errors.billing_interval}>
                {(id) => (
                  <Select id={id} value={interval} onChange={(event) => setInterval(event.target.value)}>
                    {intervals.map((option) => (
                      <option key={option.value} value={option.value}>
                        {intervalLabels[option.value]}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              {product && (
                <dl className="space-y-2 text-sm">
                  <div className="flex justify-between gap-4">
                    <dt className="text-zinc-500 dark:text-white/70">{product.name}</dt>
                    <dd className="text-zinc-800 dark:text-white">{formatEuro(product.monthly_price_cents)}</dd>
                  </div>
                  <div className="flex justify-between gap-4 border-t border-zinc-200 pt-2 font-medium dark:border-white/10">
                    <dt className="text-zinc-800 dark:text-white">{t('Te betalen')}</dt>
                    <dd className="text-zinc-800 dark:text-white">{formatEuro(product.monthly_price_cents * months)}</dd>
                  </div>
                  <p className="text-xs text-zinc-500 dark:text-white/60">{t('Inclusief btw.')}</p>
                </dl>
              )}
              {errors.product_id && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.product_id}</p>}
              <Button type="submit" variant="primary" className="w-full" disabled={!product || place.isPending}>
                {t('Bestellen en betalen')}
              </Button>
            </div>
          </Card>
        </div>
      </form>
    </>
  )
}
