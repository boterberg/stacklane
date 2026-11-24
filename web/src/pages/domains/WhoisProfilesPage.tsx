import { EnvelopeIcon, MapPinIcon, PencilSquareIcon, PhoneIcon, PlusIcon, TrashIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input, Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField } from '../../components/ui/ListToolbar'
import { ApiError } from '../../lib/api'
import { countries } from '../../lib/countries'
import { displayPhone, holderName, useDeleteWhoisProfile, useSaveWhoisProfile, useWhoisProfiles, type WhoisInput, type WhoisProfile } from '../../lib/domains'
import { t } from '../../lib/i18n'

const empty: WhoisInput = { first_name: '', last_name: '', company: '', email: '', phone: '+31 ', street: '', house_number: '', zipcode: '', city: '', country: 'NL' }

function ProfilePanel({ open, profile, onClose }: { open: boolean; profile: WhoisProfile | null; onClose: () => void }) {
  const save = useSaveWhoisProfile()
  const [form, setForm] = useState<WhoisInput>(() => (profile ? { ...profile, phone: displayPhone(profile.phone), company: profile.company ?? '' } : empty))
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const locked = (profile?.domains ?? 0) > 0
  const text = (key: keyof WhoisInput) => ({
    value: form[key] ?? '',
    invalid: !!errors[key],
    onChange: (event: { target: { value: string } }) => setForm({ ...form, [key]: event.target.value }),
  })

  function close() {
    save.reset()
    onClose()
  }

  return (
    <SlideOver
      open={open}
      onClose={close}
      title={profile ? t('WHOIS-profiel bewerken') : t('WHOIS-profiel toevoegen')}
      description={t('De houder van een domein. Deze gegevens gaan naar de registry en staan, afhankelijk van de extensie, in de openbare WHOIS.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" disabled={save.isPending} onClick={() => save.mutate({ ...form, id: profile?.id }, { onSuccess: close })}>
            {profile ? t('Opslaan') : t('Toevoegen')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        <div className="grid gap-6 sm:grid-cols-2">
          <Field label={t('Voornaam')} error={errors.first_name}>
            {(id) => <Input id={id} autoComplete="given-name" disabled={locked} {...text('first_name')} />}
          </Field>
          <Field label={t('Achternaam')} error={errors.last_name}>
            {(id) => <Input id={id} autoComplete="family-name" disabled={locked} {...text('last_name')} />}
          </Field>
        </div>
        {locked && <p className="-mt-3 text-sm text-zinc-500 dark:text-white/60">{t('De naam kun je niet wijzigen zolang er domeinen op dit profiel staan.')}</p>}
        <Field label={t('Bedrijfsnaam')} description={t('Leeg laten als het domein op jouw naam staat.')} error={errors.company}>
          {(id) => <Input id={id} autoComplete="organization" {...text('company')} />}
        </Field>
        <Field label={t('E-mailadres')} error={errors.email}>
          {(id) => <Input id={id} type="email" autoComplete="email" {...text('email')} />}
        </Field>
        <Field label={t('Telefoonnummer')} description={t('Met landcode, zoals +31 612345678.')} error={errors.phone}>
          {(id) => <Input id={id} type="tel" autoComplete="tel" {...text('phone')} />}
        </Field>
        <div className="grid gap-6 sm:grid-cols-[2fr_1fr]">
          <Field label={t('Straat')} error={errors.street}>
            {(id) => <Input id={id} autoComplete="address-line1" {...text('street')} />}
          </Field>
          <Field label={t('Huisnummer')} error={errors.house_number}>
            {(id) => <Input id={id} {...text('house_number')} />}
          </Field>
        </div>
        <div className="grid gap-6 sm:grid-cols-[1fr_2fr]">
          <Field label={t('Postcode')} error={errors.zipcode}>
            {(id) => <Input id={id} autoComplete="postal-code" {...text('zipcode')} />}
          </Field>
          <Field label={t('Plaats')} error={errors.city}>
            {(id) => <Input id={id} autoComplete="address-level2" {...text('city')} />}
          </Field>
        </div>
        <Field label={t('Land')} error={errors.country}>
          {(id) => (
            <Select id={id} value={form.country} onChange={(event) => setForm({ ...form, country: event.target.value })}>
              {countries().map((country) => (
                <option key={country.code} value={country.code}>
                  {country.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        {errors.profile && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errors.profile}</p>}
      </div>
    </SlideOver>
  )
}

export function WhoisProfilesPage() {
  const { data: profiles, isPending } = useWhoisProfiles()
  const remove = useDeleteWhoisProfile()
  const [query, setQuery] = useState('')
  const [editing, setEditing] = useState<WhoisProfile | 'new' | null>(null)
  const [deleting, setDeleting] = useState<WhoisProfile | null>(null)
  const needle = query.trim().toLowerCase()
  const shown = (profiles ?? []).filter((profile) => !needle || `${holderName(profile)} ${profile.email} ${profile.city}`.toLowerCase().includes(needle))
  const removeError = remove.error instanceof ApiError ? (remove.error.fieldErrors.profile ?? remove.error.message) : null

  return (
    <>
      <PageHeader
        title={t('WHOIS-profielen')}
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setEditing('new')}>
            {t('Profiel toevoegen')}
          </Button>
        }
      />
      <ListToolbar>
        <SearchField value={query} onChange={setQuery} placeholder={t('WHOIS-profielen zoeken...')} />
      </ListToolbar>
      {!isPending && shown.length === 0 ? (
        <EmptyState title={t('Geen whois-profielen')} description={needle ? t('Er zijn geen profielen gevonden.') : t('Je hebt nog geen WHOIS-profielen.')} />
      ) : (
        <ul className="space-y-3">
          {shown.map((profile) => (
            <li key={profile.id} className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-5 dark:border-white/10 dark:bg-white/10">
              <div className="min-w-0">
                <p className="flex items-center gap-2 text-sm font-medium text-zinc-800 dark:text-white">
                  <UserIcon className="size-4 text-zinc-400 dark:text-white/50" aria-hidden />
                  {holderName(profile)}
                </p>
                <div className="mt-2 flex flex-wrap gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
                  <span className="flex items-center gap-2">
                    <EnvelopeIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                    {profile.email}
                  </span>
                  <span className="flex items-center gap-2">
                    <PhoneIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                    {displayPhone(profile.phone)}
                  </span>
                  <span className="flex items-center gap-2">
                    <MapPinIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                    {profile.city}, {profile.country}
                  </span>
                  <span>{profile.domains === 1 ? t('1 domein') : t('{count} domeinen', { count: profile.domains })}</span>
                </div>
              </div>
              <div className="flex gap-2">
                <Button size="sm" icon={PencilSquareIcon} onClick={() => setEditing(profile)}>
                  {t('Bewerken')}
                </Button>
                <Button size="sm" icon={TrashIcon} disabled={profile.domains > 0} onClick={() => setDeleting(profile)}>
                  {t('Verwijderen')}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      {editing !== null && <ProfilePanel key={editing === 'new' ? 'new' : editing.id} open profile={editing === 'new' ? null : editing} onClose={() => setEditing(null)} />}
      <ConfirmDialog
        open={deleting !== null}
        title={t('Profiel verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setDeleting(null)}
        onConfirm={() => deleting && remove.mutate(deleting.id, { onSettled: () => setDeleting(null) })}
      >
        {removeError ?? t('Het profiel is daarna weg.')}
      </ConfirmDialog>
    </>
  )
}
