import { useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Field, Input } from '../../components/ui/Field'
import { useLogoutOtherSessions } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'

export function SessionsTab() {
  const logout = useLogoutOtherSessions()
  const [password, setPassword] = useState('')
  const error = logout.error instanceof ApiError ? logout.error.fieldErrors.password : undefined

  function submit(event: FormEvent) {
    event.preventDefault()
    logout.mutate(password, { onSuccess: () => setPassword('') })
  }

  return (
    <form onSubmit={submit} noValidate className="max-w-xl">
      <Card
        title={t('Andere apparaten')}
        description={t('Log uit op elk ander apparaat waarop je bent ingelogd. Hier blijf je gewoon ingelogd.')}
        footer={
          <>
            {logout.isSuccess && <span className="text-sm text-zinc-500 dark:text-white/70">{t('Je bent overal anders uitgelogd.')}</span>}
            <Button type="submit" variant="primary" disabled={logout.isPending || !password}>
              {t('Uitloggen')}
            </Button>
          </>
        }
      >
        <Field label={t('Je wachtwoord')} error={error}>
          {(id) => <Input id={id} type="password" autoComplete="current-password" invalid={!!error} value={password} onChange={(event) => setPassword(event.target.value)} />}
        </Field>
      </Card>
    </form>
  )
}
