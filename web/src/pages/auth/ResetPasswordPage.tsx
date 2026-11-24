import { useMutation } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'
import { useSearchParams } from 'react-router'

import { Button, ButtonLink } from '../../components/ui/Button'
import { Field, FormError, Input } from '../../components/ui/Field'
import { api, ApiError } from '../../lib/api'
import { AuthFooter, AuthHeading, TextLink } from './AuthLayout'
import { t } from '../../lib/i18n'

export function ResetPasswordPage() {
  const [params] = useSearchParams()
  const token = params.get('token') ?? ''
  const [password, setPassword] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [mismatch, setMismatch] = useState(false)
  const reset = useMutation({ mutationFn: () => api<void>('/auth/reset-password', { method: 'POST', body: { token, password } }) })
  const errors = reset.error instanceof ApiError ? reset.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    if (password !== confirmation) {
      setMismatch(true)
      return
    }
    setMismatch(false)
    reset.mutate()
  }

  if (reset.isSuccess) {
    return (
      <>
        <AuthHeading>{t('Wachtwoord ingesteld')}</AuthHeading>
        <p className="mb-6 text-center text-sm text-zinc-500 dark:text-white/70">{t('Je kunt nu inloggen met je nieuwe wachtwoord. Op andere apparaten ben je uitgelogd.')}</p>
        <ButtonLink to="/login" variant="primary" className="w-full">
          {t('Naar inloggen')}
        </ButtonLink>
      </>
    )
  }

  return (
    <>
      <AuthHeading>{t('Nieuw wachtwoord')}</AuthHeading>
      {!token || errors.token ? (
        <p className="text-center text-sm text-zinc-500 dark:text-white/70">
          {errors.token ?? t('Deze link is niet compleet.')} <TextLink to="/forgot-password">{t('Vraag een nieuwe link aan')}</TextLink>
        </p>
      ) : (
        <form onSubmit={submit} className="space-y-6" noValidate>
          <Field label={t('Nieuw wachtwoord')} description={t('Minstens 10 tekens.')} error={errors.password}>
            {(id) => <Input id={id} type="password" autoComplete="new-password" invalid={!!errors.password} value={password} onChange={(event) => setPassword(event.target.value)} />}
          </Field>
          <Field label={t('Nog een keer')} error={mismatch ? t('De wachtwoorden zijn niet gelijk.') : undefined}>
            {(id) => <Input id={id} type="password" autoComplete="new-password" invalid={mismatch} value={confirmation} onChange={(event) => setConfirmation(event.target.value)} />}
          </Field>
          <FormError error={reset.error} />
          <Button type="submit" variant="primary" className="w-full" disabled={reset.isPending}>
            {t('Wachtwoord opslaan')}
          </Button>
        </form>
      )}
      <AuthFooter>
        {t('Weet je je wachtwoord weer?')} <TextLink to="/login">{t('Inloggen')}</TextLink>
      </AuthFooter>
    </>
  )
}
