import { useMutation } from '@tanstack/react-query'
import { useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Field, Input } from '../../components/ui/Field'
import { api, ApiError } from '../../lib/api'
import { AuthFooter, AuthHeading, TextLink } from './AuthLayout'

export function ForgotPasswordPage() {
  const [email, setEmail] = useState('')
  const request = useMutation({ mutationFn: () => api<void>('/auth/forgot-password', { method: 'POST', body: { email } }) })
  const errors = request.error instanceof ApiError ? request.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    request.mutate()
  }

  return (
    <>
      <AuthHeading>Wachtwoord vergeten</AuthHeading>
      {request.isSuccess ? (
        <p className="text-center text-sm text-zinc-500 dark:text-white/70">
          Als er een account bestaat met dit e-mailadres, ontvang je binnen enkele minuten een link om je wachtwoord te herstellen.
        </p>
      ) : (
        <form onSubmit={submit} className="space-y-6" noValidate>
          <Field label="E-mailadres" error={errors.email}>
            {(id) => <Input id={id} type="email" autoComplete="email" invalid={!!errors.email} value={email} onChange={(event) => setEmail(event.target.value)} />}
          </Field>
          <Button type="submit" variant="primary" className="w-full" disabled={request.isPending}>
            Reset aanvragen
          </Button>
        </form>
      )}
      <AuthFooter>
        Weet je je wachtwoord nog? <TextLink to="/login">Inloggen</TextLink>
      </AuthFooter>
    </>
  )
}
