import { TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Field, Input } from '../../components/ui/Field'
import { useDeleteAccount } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'

export function DeleteAccountTab() {
  const remove = useDeleteAccount()
  const navigate = useNavigate()
  const [open, setOpen] = useState(false)
  const [password, setPassword] = useState('')
  const error = remove.error instanceof ApiError ? (remove.error.fieldErrors.password ?? remove.error.message) : undefined

  return (
    <div className="max-w-xl">
      <Card title={<span className="text-red-600 dark:text-red-400">{t('Gevarenzone')}</span>} description={t('Dit kun je niet ongedaan maken.')}>
        <div className="flex items-center justify-between gap-6">
          <div className="text-sm">
            <p className="font-medium text-zinc-800 dark:text-white">{t('Account verwijderen')}</p>
            <p className="mt-1 text-zinc-500 dark:text-white/70">
              {t('We wissen je persoonsgegevens en je kunt niet meer inloggen. Facturen bewaren we zolang de wet dat vraagt.')}
            </p>
          </div>
          <Button size="sm" variant="danger" icon={TrashIcon} onClick={() => setOpen(true)}>
            {t('Verwijderen')}
          </Button>
        </div>
      </Card>
      <ConfirmDialog
        open={open}
        title={t('Account verwijderen?')}
        confirmLabel={t('Account verwijderen')}
        danger
        pending={remove.isPending || !password}
        onClose={() => {
          setOpen(false)
          setPassword('')
          remove.reset()
        }}
        onConfirm={() => remove.mutate(password, { onSuccess: () => navigate('/login', { replace: true }) })}
      >
        <p>{t('Abonnementen moeten opgezegd zijn en facturen betaald.')}</p>
        <div className="mt-4">
          <Field label={t('Je wachtwoord')} error={error}>
            {(id) => <Input id={id} type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} />}
          </Field>
        </div>
      </ConfirmDialog>
    </div>
  )
}
