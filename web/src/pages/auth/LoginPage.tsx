import { useState, type FormEvent } from 'react'
import { useLocation, useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Checkbox, Field, Input } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { useAuthMutation } from '../../lib/auth'
import { AuthFooter, AuthHeading, TextLink } from './AuthLayout'

type LoginInput = { email: string; password: string; remember: boolean }

export function LoginPage() {
  const [form, setForm] = useState<LoginInput>({ email: '', password: '', remember: false })
  const login = useAuthMutation<LoginInput>('/auth/login')
  const navigate = useNavigate()
  const location = useLocation()
  const errors = login.error instanceof ApiError ? login.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    login.mutate(form, { onSuccess: () => navigate((location.state as { from?: string } | null)?.from ?? '/', { replace: true }) })
  }

  return (
    <>
      <AuthHeading>Inloggen bij Stacklane</AuthHeading>
      <form onSubmit={submit} className="space-y-6" noValidate>
        <Field label="E-mailadres" error={errors.email}>
          {(id) => (
            <Input id={id} type="email" autoComplete="email" required invalid={!!errors.email} value={form.email} onChange={(event) => setForm({ ...form, email: event.target.value })} />
          )}
        </Field>
        <Field label="Wachtwoord" error={errors.password}>
          {(id) => (
            <Input id={id} type="password" autoComplete="current-password" required value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} />
          )}
        </Field>
        <div className="flex items-center justify-between gap-4">
          <Checkbox label="Ingelogd blijven" checked={form.remember} onChange={(event) => setForm({ ...form, remember: event.target.checked })} />
          <TextLink to="/forgot-password">Wachtwoord vergeten?</TextLink>
        </div>
        <Button type="submit" variant="primary" className="w-full" disabled={login.isPending}>
          Inloggen
        </Button>
      </form>
      <AuthFooter>
        Nog geen account? <TextLink to="/register">Maak een account aan</TextLink>
      </AuthFooter>
    </>
  )
}
