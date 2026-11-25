import { Radio, RadioGroup } from '@headlessui/react'
import { useState, type FormEvent } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { cn } from '../../lib/cn'
import { formatMoney } from '../../lib/format'
import { t } from '../../lib/i18n'
import { intervalLabels } from '../../lib/labels'
import { specsLine, usePlaceOrder, useShopProducts, useSshKeys } from '../../lib/vps'

const intervals = [
  { value: 'monthly', months: 1 },
  { value: 'quarterly', months: 3 },
  { value: 'half_yearly', months: 6 },
  { value: 'yearly', months: 12 },
]

export function OrderServerPage() {
  const { data: products, isPending } = useShopProducts('vps')
  const { data: keys } = useSshKeys()
  const place = usePlaceOrder()
  const navigate = useNavigate()
  const [searchParams] = useSearchParams()
  const [productId, setProductId] = useState<string | null>(searchParams.get('product'))
  const [interval, setInterval] = useState('monthly')
  const [name, setName] = useState('')
  const [image, setImage] = useState<string | null>(null)
  const [keyIds, setKeyIds] = useState<string[]>([])
  const errors = place.error instanceof ApiError ? place.error.fieldErrors : {}

  const product = (products ?? []).find((candidate) => candidate.id === productId) ?? products?.[0]
  const chosenImage = image ?? product?.images[0]?.id ?? null
  const months = intervals.find((option) => option.value === interval)?.months ?? 1

  function submit(event: FormEvent) {
    event.preventDefault()
    if (!product) return
    place.mutate(
      { product_id: product.id, billing_interval: interval, name, image: chosenImage, ssh_key_ids: keyIds },
      { onSuccess: (order) => navigate(`/billing/invoices/${order.invoice_id}`) },
    )
  }

  if (!isPending && (products ?? []).length === 0) {
    return (
      <>
        <PageHeader title={t('Server bestellen')} />
        <EmptyState title={t('Geen pakketten')} description={t('Er zijn nog geen VPS-pakketten te bestellen.')} />
      </>
    )
  }

  return (
    <>
      <PageHeader title={t('Server bestellen')} description={t('Kies een pakket. Na betaling staat je server binnen een paar minuten klaar.')} />
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
                  <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{specsLine(option.specs)}</p>
                  <p className="mt-3 text-sm font-medium text-zinc-800 dark:text-white">{t('{price} per maand', { price: formatMoney(option.monthly_price_cents) })}</p>
                </Radio>
              ))}
            </RadioGroup>
          </Card>
          <Card title={t('Server')}>
            <div className="space-y-6">
              <Field label={t('Naam')} description={t('Wordt ook de hostname, bijvoorbeeld web-01.')} error={errors.name}>
                {(id) => <Input id={id} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
              </Field>
              {product && product.images.length > 0 && (
                <Field label={t('Besturingssysteem')} error={errors.image}>
                  {(id) => (
                    <Select id={id} value={chosenImage ?? ''} onChange={(event) => setImage(event.target.value)}>
                      {product.images.map((option) => (
                        <option key={option.id} value={option.id}>
                          {option.name}
                        </option>
                      ))}
                    </Select>
                  )}
                </Field>
              )}
              <div>
                <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('SSH-sleutels')}</p>
                {(keys ?? []).length === 0 ? (
                  <p className="mt-2 text-sm text-zinc-500 dark:text-white/60">
                    {t('Je hebt nog geen SSH-sleutels. Je kunt ook inloggen met het rootwachtwoord.')}{' '}
                    <Link to="/vps/ssh-keys" className="font-medium text-zinc-800 underline-offset-4 hover:underline dark:text-white">
                      {t('Sleutel toevoegen')}
                    </Link>
                  </p>
                ) : (
                  <div className="mt-3 space-y-2">
                    {(keys ?? []).map((key) => (
                      <Checkbox
                        key={key.id}
                        className="flex"
                        label={key.name}
                        checked={keyIds.includes(key.id)}
                        onChange={(event) => setKeyIds(event.target.checked ? [...keyIds, key.id] : keyIds.filter((id) => id !== key.id))}
                      />
                    ))}
                  </div>
                )}
              </div>
            </div>
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
                    <dd className="text-zinc-800 dark:text-white">{formatMoney(product.monthly_price_cents)}</dd>
                  </div>
                  <div className="flex justify-between gap-4 border-t border-zinc-200 pt-2 font-medium dark:border-white/10">
                    <dt className="text-zinc-800 dark:text-white">{t('Te betalen')}</dt>
                    <dd className="text-zinc-800 dark:text-white">{formatMoney(product.monthly_price_cents * months)}</dd>
                  </div>
                  <p className="text-xs text-zinc-500 dark:text-white/60">{t('Inclusief btw.')}</p>
                </dl>
              )}
              {(errors.product_id || errors.ssh_key_ids) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.product_id ?? errors.ssh_key_ids}</p>}
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
