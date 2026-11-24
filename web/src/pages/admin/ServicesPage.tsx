import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { EllipsisHorizontalIcon, GlobeAltIcon, PlusIcon, ServerIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { formatRelative } from '../../lib/format'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { useCreateService, useCustomerSearch, useProducts, useServiceAction, useServices, type Service } from '../../lib/admin'

const emptyForm = { customer_email: '', product_id: '', name: '' }

function CreateServicePanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [form, setForm] = useState(emptyForm)
  const { data: products = [] } = useProducts()
  const { data: customers = [] } = useCustomerSearch(form.customer_email)
  const create = useCreateService()
  const errors = create.error instanceof ApiError ? create.error.fieldErrors : {}

  function close() {
    onClose()
    create.reset()
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate(form, {
      onSuccess: () => {
        setForm(emptyForm)
        close()
      },
    })
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title="Dienst aanmaken"
      description="Stacklane maakt de dienst meteen aan op de koppeling van het product. Facturatie volgt in een volgende versie."
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            Annuleren
          </Button>
          <Button variant="primary" type="submit" form="create-service" disabled={create.isPending}>
            Aanmaken
          </Button>
        </>
      }
    >
      <form id="create-service" onSubmit={submit} className="space-y-6" noValidate>
        <Field label="Klant" description="Zoek op naam of e-mailadres." error={errors.customer_email}>
          {(id) => (
            <>
              <Input id={id} list="customer-options" value={form.customer_email} invalid={!!errors.customer_email} onChange={(event) => setForm({ ...form, customer_email: event.target.value })} />
              <datalist id="customer-options">
                {customers.map((customer) => (
                  <option key={customer.id} value={customer.email}>
                    {customer.name}
                  </option>
                ))}
              </datalist>
            </>
          )}
        </Field>
        <Field label="Product" error={errors.product_id}>
          {(id) => (
            <Select id={id} value={form.product_id} onChange={(event) => setForm({ ...form, product_id: event.target.value })}>
              <option value="">Kies een product...</option>
              {products.map((product) => (
                <option key={product.id} value={product.id}>
                  {product.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label="Naam van de server" description="De klant ziet deze naam in zijn panel." error={errors.name}>
          {(id) => <Input id={id} placeholder="Survival" maxLength={60} value={form.name} invalid={!!errors.name} onChange={(event) => setForm({ ...form, name: event.target.value })} />}
        </Field>
      </form>
    </SlideOver>
  )
}

function Actions({ service }: { service: Service }) {
  const action = useServiceAction()
  const items: Array<{ action: 'suspend' | 'unsuspend' | 'terminate' | 'retry'; label: string }> = [
    ...(service.status === 'active' ? [{ action: 'suspend' as const, label: 'Opschorten' }] : []),
    ...(service.status === 'suspended' ? [{ action: 'unsuspend' as const, label: 'Heractiveren' }] : []),
    ...(service.status === 'failed' ? [{ action: 'retry' as const, label: 'Opnieuw proberen' }] : []),
    ...(['active', 'suspended', 'failed', 'pending'].includes(service.status) ? [{ action: 'terminate' as const, label: 'Opheffen' }] : []),
  ]
  if (items.length === 0) return null
  return (
    <Menu>
      <MenuButton className="rounded-md p-1.5 text-zinc-400 hover:bg-zinc-800/5 hover:text-zinc-800 dark:hover:bg-white/10 dark:hover:text-white" aria-label="Acties">
        <EllipsisHorizontalIcon className="size-5" />
      </MenuButton>
      <MenuItems anchor={{ to: 'bottom end', gap: 4 }} className="z-50 min-w-44 rounded-lg border border-zinc-200 bg-white p-1 shadow-lg focus:outline-none dark:border-zinc-600 dark:bg-zinc-700">
        {items.map((item) => (
          <MenuItem key={item.action}>
            <button
              type="button"
              onClick={() => action.mutate({ id: service.id, action: item.action })}
              className={`flex w-full rounded-md px-3 py-1.5 text-left text-sm font-medium data-focus:bg-zinc-100 dark:data-focus:bg-zinc-600 ${item.action === 'terminate' ? 'text-red-600 dark:text-red-400' : 'text-zinc-800 dark:text-white'}`}
            >
              {item.label}
            </button>
          </MenuItem>
        ))}
      </MenuItems>
    </Menu>
  )
}

export function ServicesPage() {
  const { data: services, isPending } = useServices()
  const [creating, setCreating] = useState(false)

  return (
    <>
      <PageHeader
        title="Diensten"
        description="Alle diensten van je klanten en de staat waarin ze zijn."
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setCreating(true)}>
            Dienst aanmaken
          </Button>
        }
      />
      {!isPending && (services ?? []).length === 0 ? (
        <EmptyState title="Nog geen diensten" description="Maak een dienst aan voor een klant." />
      ) : (
        <ul className="space-y-3">
          {(services ?? []).map((service) => (
            <li key={service.id} className="flex items-start gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-5 dark:border-white/10 dark:bg-white/10">
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-3">
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{service.name}</p>
                  <Badge color={serviceStatusColors[service.status]}>{serviceStatusLabels[service.status]}</Badge>
                </div>
                <div className="mt-2 flex flex-wrap gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
                  <span className="flex items-center gap-2">
                    <UserIcon className="size-4 text-zinc-400 dark:text-white/40" />
                    {service.customer_name}
                  </span>
                  {service.product_name && (
                    <span className="flex items-center gap-2">
                      <ServerIcon className="size-4 text-zinc-400 dark:text-white/40" />
                      {service.product_name}
                    </span>
                  )}
                  {service.details.address && (
                    <span className="flex items-center gap-2">
                      <GlobeAltIcon className="size-4 text-zinc-400 dark:text-white/40" />
                      {service.details.address}
                    </span>
                  )}
                  <span>{formatRelative(service.created_at)}</span>
                </div>
                {service.last_error && <p className="mt-2 text-sm text-red-600 dark:text-red-400">{service.last_error}</p>}
              </div>
              <Actions service={service} />
            </li>
          ))}
        </ul>
      )}
      <CreateServicePanel open={creating} onClose={() => setCreating(false)} />
    </>
  )
}
