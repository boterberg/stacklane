import { CpuChipIcon, CircleStackIcon, CurrencyEuroIcon, LinkIcon, PlusIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { formatEuro, formatMegabytes } from '../../lib/format'
import { useCatalog, useCreateProduct, useIntegrations, useProducts } from '../../lib/admin'

const emptyForm = {
  name: '',
  price: '',
  integration_id: '',
  location_id: '',
  nest_id: '',
  egg_id: '',
  memory: '2048',
  disk: '10240',
  cpu: '100',
  databases: '1',
  backups: '2',
}

function CreateProductPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [form, setForm] = useState(emptyForm)
  const { data: integrations = [] } = useIntegrations()
  const { data: catalog, isFetching } = useCatalog(form.integration_id || null)
  const create = useCreateProduct()
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}
  const eggs = catalog?.nests.find((nest) => String(nest.id) === form.nest_id)?.eggs ?? []

  function update(key: keyof typeof emptyForm, value: string) {
    setForm((current) => ({ ...current, [key]: value, ...(key === 'nest_id' ? { egg_id: '' } : {}), ...(key === 'integration_id' ? { nest_id: '', egg_id: '', location_id: '' } : {}) }))
  }

  function close() {
    onClose()
    create.reset()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate(
      {
        kind: 'gameserver',
        name: form.name,
        monthly_price_cents: Math.round(Number(form.price.replace(',', '.')) * 100) || 0,
        integration_id: form.integration_id || null,
        provisioning: {
          location_id: Number(form.location_id),
          nest_id: Number(form.nest_id),
          egg_id: Number(form.egg_id),
          memory: Number(form.memory),
          disk: Number(form.disk),
          cpu: Number(form.cpu),
          databases: Number(form.databases),
          backups: Number(form.backups),
          allocations: 1,
        },
      },
      {
        onSuccess: () => {
          setForm(emptyForm)
          close()
        },
      },
    )
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title="Product aanmaken"
      description="Een gameserverpakket dat je aan klanten kunt leveren."
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            Annuleren
          </Button>
          <Button variant="primary" type="submit" form="create-product" disabled={create.isPending}>
            Aanmaken
          </Button>
        </>
      }
    >
      <form id="create-product" onSubmit={submit} className="space-y-6" noValidate>
        <div className="grid gap-6 sm:grid-cols-[2fr_1fr]">
          <Field label="Naam" error={errors.name}>
            {(id) => <Input id={id} placeholder="Minecraft 4 GB" value={form.name} invalid={!!errors.name} onChange={(event) => update('name', event.target.value)} />}
          </Field>
          <Field label="Prijs per maand" error={errors.monthly_price_cents}>
            {(id) => <Input id={id} inputMode="decimal" placeholder="12,50" value={form.price} onChange={(event) => update('price', event.target.value)} />}
          </Field>
        </div>
        <Field label="Koppeling" error={errors.integration_id}>
          {(id) => (
            <Select id={id} value={form.integration_id} onChange={(event) => update('integration_id', event.target.value)}>
              <option value="">Kies een Pterodactyl-panel...</option>
              {integrations
                .filter((integration) => integration.kind === 'pterodactyl')
                .map((integration) => (
                  <option key={integration.id} value={integration.id}>
                    {integration.name}
                  </option>
                ))}
            </Select>
          )}
        </Field>
        {form.integration_id && (
          <>
            {isFetching && <p className="text-sm text-zinc-500">Locaties en eggs ophalen...</p>}
            <div className="grid gap-6 sm:grid-cols-3">
              <Field label="Locatie">
                {(id) => (
                  <Select id={id} value={form.location_id} onChange={(event) => update('location_id', event.target.value)}>
                    <option value="">Kies...</option>
                    {catalog?.locations.map((location) => (
                      <option key={location.id} value={location.id}>
                        {location.name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label="Nest">
                {(id) => (
                  <Select id={id} value={form.nest_id} onChange={(event) => update('nest_id', event.target.value)}>
                    <option value="">Kies...</option>
                    {catalog?.nests.map((nest) => (
                      <option key={nest.id} value={nest.id}>
                        {nest.name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label="Egg">
                {(id) => (
                  <Select id={id} value={form.egg_id} disabled={!form.nest_id} onChange={(event) => update('egg_id', event.target.value)}>
                    <option value="">Kies...</option>
                    {eggs.map((egg) => (
                      <option key={egg.id} value={egg.id}>
                        {egg.name}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
            </div>
          </>
        )}
        <div className="grid gap-6 sm:grid-cols-3">
          <Field label="Geheugen (MB)">{(id) => <Input id={id} inputMode="numeric" value={form.memory} onChange={(event) => update('memory', event.target.value)} />}</Field>
          <Field label="Opslag (MB)">{(id) => <Input id={id} inputMode="numeric" value={form.disk} onChange={(event) => update('disk', event.target.value)} />}</Field>
          <Field label="Processor (%)">{(id) => <Input id={id} inputMode="numeric" value={form.cpu} onChange={(event) => update('cpu', event.target.value)} />}</Field>
        </div>
        <div className="grid gap-6 sm:grid-cols-3">
          <Field label="Databases">{(id) => <Input id={id} inputMode="numeric" value={form.databases} onChange={(event) => update('databases', event.target.value)} />}</Field>
          <Field label="Back-ups">{(id) => <Input id={id} inputMode="numeric" value={form.backups} onChange={(event) => update('backups', event.target.value)} />}</Field>
        </div>
        {errors.provisioning && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.provisioning}</p>}
      </form>
    </SlideOver>
  )
}

export function ProductsPage() {
  const { data: products, isPending } = useProducts()
  const [creating, setCreating] = useState(false)

  return (
    <>
      <PageHeader
        title="Producten"
        description="De pakketten die je levert. Elk product hoort bij een koppeling."
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            Product aanmaken
          </Button>
        }
      />
      {!isPending && (products ?? []).length === 0 ? (
        <EmptyState title="Nog geen producten" description="Maak eerst een koppeling aan en daarna een product." />
      ) : (
        <ul className="space-y-3">
          {(products ?? []).map((product) => (
            <li key={product.id} className="rounded-xl border border-zinc-200 bg-white px-6 py-5 dark:border-white/10 dark:bg-white/10">
              <p className="text-sm font-medium text-zinc-800 dark:text-white">{product.name}</p>
              <div className="mt-2 flex flex-wrap gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
                <span className="flex items-center gap-2">
                  <CurrencyEuroIcon className="size-4 text-zinc-400 dark:text-white/40" />
                  {formatEuro(product.monthly_price_cents)} per maand
                </span>
                {product.provisioning.memory && (
                  <span className="flex items-center gap-2">
                    <CpuChipIcon className="size-4 text-zinc-400 dark:text-white/40" />
                    {formatMegabytes(product.provisioning.memory)} · {product.provisioning.cpu}% CPU
                  </span>
                )}
                {product.provisioning.disk && (
                  <span className="flex items-center gap-2">
                    <CircleStackIcon className="size-4 text-zinc-400 dark:text-white/40" />
                    {formatMegabytes(product.provisioning.disk)}
                  </span>
                )}
                {product.integration_name && (
                  <span className="flex items-center gap-2">
                    <LinkIcon className="size-4 text-zinc-400 dark:text-white/40" />
                    {product.integration_name}
                  </span>
                )}
                <span>{product.service_count === 1 ? '1 dienst' : `${product.service_count} diensten`}</span>
              </div>
            </li>
          ))}
        </ul>
      )}
      <CreateProductPanel open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
