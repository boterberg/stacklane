import { useState, type FormEvent } from 'react'
import { useLocation, useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Checkbox, Field, FormError, Input } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { useAuthMutation } from '../../lib/auth'
import { useDemoLogin, useMeta, type DemoRole } from '../../lib/meta'
import { AuthFooter, AuthHeading, TextLink } from './AuthLayout'
import { t } from '../../lib/i18n'

type LoginInput = { email: string; password: string; remember: boolean; code?: string }

export function LoginPage() {
  const [form, setForm] = useState<LoginInput>({ email: '', password: '', remember: false })
  const login = useAuthMutation<LoginInput>('/auth/login')
  const navigate = useNavigate()
  const location = useLocation()
  const errors = login.error instanceof ApiError ? login.error.fieldErrors : {}
  const { data: meta } = useMeta()
  const demoLogin = useDemoLogin()
  const [needsCode, setNeedsCode] = useState(false)
  const [code, setCode] = useState('')

  function enterDemo(role: DemoRole) {
    demoLogin.mutate(role, { onSuccess: () => navigate(role === 'admin' ? '/admin/tickets' : '/', { replace: true }) })
  }

  function submit(event: FormEvent) {
    event.preventDefault()
    login.mutate(needsCode ? { ...form, code } : form, {
      onSuccess: () => navigate((location.state as { from?: string } | null)?.from ?? '/', { replace: true }),
      onError: (error) => {
        if (!needsCode && error instanceof ApiError && error.fieldErrors.code) {
          setNeedsCode(true)
          login.reset()
        }
      },
    })
  }

  if (needsCode) {
    return (
      <>
        <AuthHeading>{t('Tweestapsverificatie')}</AuthHeading>
        <form onSubmit={submit} className="space-y-6" noValidate>
          <Field label={t('Code')} description={t('Open je authenticator-app en vul de code van zes cijfers in. Telefoon kwijt? Gebruik een van je herstelcodes.')} error={errors.code}>
            {(id) => (
              <Input id={id} autoFocus inputMode="numeric" autoComplete="one-time-code" invalid={!!errors.code} value={code} onChange={(event) => setCode(event.target.value)} />
            )}
          </Field>
          <FormError error={login.error} />
          <Button type="submit" variant="primary" className="w-full" disabled={login.isPending || !code.trim()}>
            {t('Inloggen')}
          </Button>
        </form>
        <AuthFooter>
          <button
            type="button"
            className="font-medium text-zinc-800 underline-offset-4 hover:underline dark:text-white"
            onClick={() => {
              setNeedsCode(false)
              setCode('')
              login.reset()
            }}
          >
            {t('Terug')}
          </button>
        </AuthFooter>
      </>
    )
  }

  return (
    <>
      <AuthHeading>{t('Inloggen bij {name}', { name: meta?.branding.name ?? 'Stacklane' })}</AuthHeading>
      {meta?.demo && (
        <div className="mb-8">
          <div className="rounded-lg border border-zinc-200 bg-zinc-50 p-4 dark:border-white/10 dark:bg-white/5">
            <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Bekijk de demo')}</p>
            <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('Log in met één klik. Alles wat je verandert, wordt elk uur teruggezet.')}</p>
            <div className="mt-4 grid gap-3 sm:grid-cols-2">
              <Button variant="primary" disabled={demoLogin.isPending} onClick={() => enterDemo('customer')}>
                {t('Inloggen als klant')}
              </Button>
              <Button disabled={demoLogin.isPending} onClick={() => enterDemo('admin')}>
                {t('Inloggen als beheerder')}
              </Button>
            </div>
            {demoLogin.error && <p className="mt-3 text-sm font-medium text-red-500 dark:text-red-400">{demoLogin.error.message}</p>}
          </div>
          <div className="mt-8 flex items-center gap-3 text-xs text-zinc-400">
            <span className="h-px flex-1 bg-zinc-200 dark:bg-white/10" />
            {t('of met je eigen account')}
            <span className="h-px flex-1 bg-zinc-200 dark:bg-white/10" />
          </div>
        </div>
      )}
      <form onSubmit={submit} className="space-y-6" noValidate>
        <Field label={t('E-mailadres')} error={errors.email}>
          {(id) => (
            <Input id={id} type="email" autoComplete="email" required invalid={!!errors.email} value={form.email} onChange={(event) => setForm({ ...form, email: event.target.value })} />
          )}
        </Field>
        <Field label={t('Wachtwoord')} error={errors.password}>
          {(id) => (
            <Input id={id} type="password" autoComplete="current-password" required value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} />
          )}
        </Field>
        <div className="flex items-center justify-between gap-4">
          <Checkbox label={t('Ingelogd blijven')} checked={form.remember} onChange={(event) => setForm({ ...form, remember: event.target.checked })} />
          <TextLink to="/forgot-password">{t('Wachtwoord vergeten?')}</TextLink>
        </div>
        <FormError error={login.error} />
        <Button type="submit" variant="primary" className="w-full" disabled={login.isPending}>
          {t('Inloggen')}
        </Button>
      </form>
      {!meta?.demo && (
        <AuthFooter>
          {t('Nog geen account?')} <TextLink to="/register">{t('Maak een account aan')}</TextLink>
        </AuthFooter>
      )}
    </>
  )
}
