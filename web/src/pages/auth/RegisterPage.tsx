import { useState, type FormEvent } from 'react'
import { useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Checkbox, Field, FormError, Input, Select } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { useAuthMutation } from '../../lib/auth'
import { countries } from '../../lib/countries'
import { AuthFooter, AuthHeading, TextLink } from './AuthLayout'
import { t } from '../../lib/i18n'

type RegisterInput = {
  name: string
  email: string
  password: string
  country: string
  is_business: boolean
  company_name: string
}

export function RegisterPage() {
  const [form, setForm] = useState<RegisterInput>({ name: '', email: '', password: '', country: 'NL', is_business: false, company_name: '' })
  const register = useAuthMutation<RegisterInput>('/auth/register')
  const navigate = useNavigate()
  const errors = register.error instanceof ApiError ? register.error.fieldErrors : {}

  function update<K extends keyof RegisterInput>(key: K, value: RegisterInput[K]) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    register.mutate(form, { onSuccess: () => navigate('/', { replace: true }) })
  }

  return (
    <>
      <AuthHeading>{t('Maak een Stacklane account')}</AuthHeading>
      <form onSubmit={submit} className="space-y-6" noValidate>
        <Field label={t('Naam')} error={errors.name}>
          {(id) => <Input id={id} autoComplete="name" invalid={!!errors.name} value={form.name} onChange={(event) => update('name', event.target.value)} />}
        </Field>
        <Field label={t('E-mailadres')} error={errors.email}>
          {(id) => <Input id={id} type="email" autoComplete="email" invalid={!!errors.email} value={form.email} onChange={(event) => update('email', event.target.value)} />}
        </Field>
        <Field label={t('Wachtwoord')} error={errors.password}>
          {(id) => (
            <Input id={id} type="password" autoComplete="new-password" invalid={!!errors.password} value={form.password} onChange={(event) => update('password', event.target.value)} />
          )}
        </Field>
        <Field label={t('Land')} error={errors.country}>
          {(id) => (
            <Select id={id} autoComplete="country" value={form.country} onChange={(event) => update('country', event.target.value)}>
              {countries().map((country) => (
                <option key={country.code} value={country.code}>
                  {country.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Checkbox label={t('Ik ben een zakelijke klant')} checked={form.is_business} onChange={(event) => update('is_business', event.target.checked)} />
        {form.is_business && (
          <Field label={t('Bedrijfsnaam')} error={errors.company_name}>
            {(id) => (
              <Input id={id} autoComplete="organization" invalid={!!errors.company_name} value={form.company_name} onChange={(event) => update('company_name', event.target.value)} />
            )}
          </Field>
        )}
        <FormError error={register.error} />
        <Button type="submit" variant="primary" className="w-full" disabled={register.isPending}>
          {t('Account maken')}
        </Button>
      </form>
      <AuthFooter>
        {t('Heb je al een account?')} <TextLink to="/login">{t('Inloggen')}</TextLink>
      </AuthFooter>
    </>
  )
}
