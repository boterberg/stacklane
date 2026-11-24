import { useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Checkbox, Field, Input } from '../../components/ui/Field'
import { useChangePassword } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'

export function PasswordTab() {
  const change = useChangePassword()
  const [form, setForm] = useState({ current: '', password: '', confirm: '' })
  const [mismatch, setMismatch] = useState(false)
  const [revokeTokens, setRevokeTokens] = useState(true)
  const errors = change.error instanceof ApiError ? change.error.fieldErrors : {}

  function submit(event: FormEvent) {
    event.preventDefault()
    const differs = form.password !== form.confirm
    setMismatch(differs)
    if (differs) return
    change.mutate({ current_password: form.current, password: form.password, revoke_tokens: revokeTokens }, { onSuccess: () => setForm({ current: '', password: '', confirm: '' }) })
  }

  return (
    <form onSubmit={submit} noValidate className="max-w-xl">
      <Card
        title={t('Wijzig je wachtwoord')}
        description={t('Andere apparaten worden daarna uitgelogd.')}
        footer={
          <>
            {change.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Je wachtwoord is gewijzigd.')}</span>}
            <Button type="submit" variant="primary" disabled={change.isPending}>
              {t('Wachtwoord wijzigen')}
            </Button>
          </>
        }
      >
        <div className="space-y-6">
          <Field label={t('Huidig wachtwoord')} error={errors.current_password}>
            {(id) => (
              <Input id={id} type="password" autoComplete="current-password" invalid={!!errors.current_password} value={form.current} onChange={(event) => setForm({ ...form, current: event.target.value })} />
            )}
          </Field>
          <Field label={t('Nieuw wachtwoord')} error={errors.password}>
            {(id) => (
              <>
                <Input id={id} type="password" autoComplete="new-password" invalid={!!errors.password} value={form.password} onChange={(event) => setForm({ ...form, password: event.target.value })} />
                <p className="mt-3 text-sm text-zinc-500 dark:text-white/60">{t('Minstens 10 tekens.')}</p>
              </>
            )}
          </Field>
          <Field label={t('Nieuw wachtwoord bevestigen')} error={mismatch ? t('De wachtwoorden zijn niet hetzelfde.') : undefined}>
            {(id) => <Input id={id} type="password" autoComplete="new-password" invalid={mismatch} value={form.confirm} onChange={(event) => setForm({ ...form, confirm: event.target.value })} />}
          </Field>
          <Checkbox label={t('Trek ook al je API-tokens in')} checked={revokeTokens} onChange={(event) => setRevokeTokens(event.target.checked)} />
        </div>
      </Card>
    </form>
  )
}
