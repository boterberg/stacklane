import { BuildingOfficeIcon, EnvelopeIcon, KeyIcon, ShieldCheckIcon, IdentificationIcon, MapPinIcon, PhoneIcon, ReceiptPercentIcon, UserIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox, Field, Input, Select } from '../../components/ui/Field'
import { useAccount, useCancelEmailChange, useResendVerification, useSaveAccount, type AccountDetails } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { useMe } from '../../lib/auth'
import { countries } from '../../lib/countries'
import { t } from '../../lib/i18n'

function ProfileForm({ initial }: { initial: AccountDetails }) {
  const [form, setForm] = useState(initial)
  const save = useSaveAccount()
  const { data: profile } = useMe()
  const resend = useResendVerification()
  const cancel = useCancelEmailChange()
  const errors = save.error instanceof ApiError ? save.error.fieldErrors : {}
  const changingEmail = form.email.trim().toLowerCase() !== initial.email.toLowerCase()
  const askCode = changingEmail && (!!errors.code || !!form.code)
  const text = (key: keyof AccountDetails) => ({
    value: (form[key] as string | null) ?? '',
    invalid: !!errors[key],
    onChange: (event: { target: { value: string } }) => setForm({ ...form, [key]: event.target.value }),
  })

  function submit(event: FormEvent) {
    event.preventDefault()
    save.mutate(form, {
      onSuccess: (account) => setForm({ ...account, current_password: '', code: '' }),
    })
  }

  const footer = (
    <>
      {save.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Opgeslagen')}</span>}
      <Button type="submit" variant="primary" disabled={save.isPending}>
        {t('Wijzigingen opslaan')}
      </Button>
    </>
  )

  return (
    <form onSubmit={submit} noValidate className="max-w-xl space-y-6">
      <Card title={t('Accountgegevens')} description={t('Beheer je accountgegevens.')}>
        <div className="space-y-6">
          <Field label={t('Voornaam')} error={errors.first_name}>
            {(id) => <Input id={id} icon={UserIcon} autoComplete="given-name" {...text('first_name')} />}
          </Field>
          <Field label={t('Achternaam')} error={errors.last_name}>
            {(id) => <Input id={id} icon={UserIcon} autoComplete="family-name" {...text('last_name')} />}
          </Field>
          <Checkbox label={t('Ik bestel zakelijk')} checked={form.is_business} onChange={(event) => setForm({ ...form, is_business: event.target.checked })} />
          {form.is_business && (
            <>
              <Field label={t('Bedrijfsnaam')} error={errors.company_name}>
                {(id) => <Input id={id} icon={BuildingOfficeIcon} autoComplete="organization" {...text('company_name')} />}
              </Field>
              <Field label={t('KVK-nummer')} error={errors.coc_number}>
                {(id) => <Input id={id} icon={IdentificationIcon} {...text('coc_number')} />}
              </Field>
              <Field
                label={t('Btw-nummer')}
                description={
                  form.tax_id
                    ? form.tax_id_verified
                      ? t('Gecontroleerd bij VIES. Je krijgt facturen met verlegde btw als je buiten ons land zit.')
                      : t('Nog niet gecontroleerd. We proberen het opnieuw bij de volgende facturatie.')
                    : undefined
                }
                error={errors.tax_id}
              >
                {(id) => <Input id={id} icon={ReceiptPercentIcon} placeholder="NL123456789B01" {...text('tax_id')} />}
              </Field>
            </>
          )}
          <Field
            label={t('E-mailadres')}
            description={changingEmail ? t('We sturen een link naar het nieuwe adres. Het verandert pas als je op die link klikt.') : undefined}
            error={errors.email}
          >
            {(id) => <Input id={id} type="email" icon={EnvelopeIcon} autoComplete="email" {...text('email')} />}
          </Field>
          {profile?.pending_email && !changingEmail && (
            <div className="rounded-lg border border-zinc-200 bg-zinc-50 px-4 py-3 text-sm text-zinc-600 dark:border-white/10 dark:bg-white/5 dark:text-white/70">
              <p>{t('We hebben een link gestuurd naar {email}. Klik erop om dit adres te gebruiken.', { email: profile.pending_email })}</p>
              <div className="mt-2 flex gap-4">
                <button type="button" disabled={resend.isPending} onClick={() => resend.mutate()} className="font-medium text-zinc-800 hover:underline dark:text-white">
                  {resend.isSuccess ? t('Opnieuw gestuurd') : t('Opnieuw sturen')}
                </button>
                <button type="button" disabled={cancel.isPending} onClick={() => cancel.mutate()} className="font-medium text-zinc-800 hover:underline dark:text-white">
                  {t('Wijziging annuleren')}
                </button>
              </div>
            </div>
          )}
          {changingEmail && (
            <Field label={t('Huidig wachtwoord')} description={t('Nodig om je e-mailadres te wijzigen.')} error={errors.current_password}>
              {(id) => <Input id={id} type="password" icon={KeyIcon} autoComplete="current-password" {...text('current_password')} />}
            </Field>
          )}
          {askCode && (
            <Field label={t('Code uit je authenticator-app')} error={errors.code}>
              {(id) => <Input id={id} icon={ShieldCheckIcon} inputMode="numeric" autoComplete="one-time-code" {...text('code')} />}
            </Field>
          )}
          <Field label={t('E-mailadres voor facturen')} description={t('Laat leeg om facturen naar je gewone e-mailadres te sturen.')} error={errors.invoice_email}>
            {(id) => <Input id={id} type="email" icon={EnvelopeIcon} placeholder={form.email} {...text('invoice_email')} />}
          </Field>
          <Field label={t('Telefoonnummer')} error={errors.phone}>
            {(id) => <Input id={id} type="tel" icon={PhoneIcon} autoComplete="tel" {...text('phone')} />}
          </Field>
        </div>
      </Card>
      <Card title={t('Adres')} description={t('Dit adres zetten we op je facturen.')} footer={footer}>
        <div className="space-y-6">
          <Field label={t('Straat en huisnummer')} error={errors.address_line1}>
            {(id) => <Input id={id} icon={MapPinIcon} autoComplete="address-line1" {...text('address_line1')} />}
          </Field>
          <Field label={t('Adresregel 2')} error={errors.address_line2}>
            {(id) => <Input id={id} autoComplete="address-line2" {...text('address_line2')} />}
          </Field>
          <div className="grid grid-cols-1 gap-6 sm:grid-cols-[10rem_1fr]">
            <Field label={t('Postcode')} error={errors.postal_code}>
              {(id) => <Input id={id} autoComplete="postal-code" {...text('postal_code')} />}
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
        </div>
      </Card>
    </form>
  )
}

export function ProfileTab() {
  const { data } = useAccount()
  return data ? <ProfileForm initial={data} /> : null
}
