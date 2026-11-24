import { CheckIcon, ClipboardDocumentIcon, ShieldCheckIcon } from '@heroicons/react/24/outline'
import { useEffect, useState, type FormEvent } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Field, Input } from '../../components/ui/Field'
import { useDisableTwoFactor, useEnableTwoFactor, useNewRecoveryCodes, useTwoFactor, useTwoFactorSetup } from '../../lib/account'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'

function fieldError(error: Error | null, field: string) {
  return error instanceof ApiError ? (error.fieldErrors[field] ?? error.message) : undefined
}

function codeError(error: Error | null) {
  return error instanceof ApiError ? error.fieldErrors.code : undefined
}

function RecoveryCodes({ codes, onDone }: { codes: string[]; onDone: () => void }) {
  const [copied, setCopied] = useState(false)
  async function copy() {
    await navigator.clipboard.writeText(codes.join('\n'))
    setCopied(true)
  }
  return (
    <Card
      title={t('Bewaar je herstelcodes')}
      description={t('Ben je je telefoon kwijt, dan log je in met een van deze codes. Elke code werkt één keer. Je ziet ze maar nu.')}
      footer={
        <>
          <Button icon={copied ? CheckIcon : ClipboardDocumentIcon} onClick={copy}>
            {copied ? t('Gekopieerd') : t('Kopiëren')}
          </Button>
          <Button variant="primary" onClick={onDone}>
            {t('Ik heb ze bewaard')}
          </Button>
        </>
      }
    >
      <ul className="grid grid-cols-2 gap-x-6 gap-y-2 font-mono text-sm text-zinc-800 dark:text-white">
        {codes.map((code) => (
          <li key={code}>{code}</li>
        ))}
      </ul>
    </Card>
  )
}

function Setup({ onCancel, onEnabled }: { onCancel: () => void; onEnabled: (codes: string[]) => void }) {
  const setup = useTwoFactorSetup()
  const enable = useEnableTwoFactor()
  const [code, setCode] = useState('')
  const start = setup.mutate
  useEffect(() => start(), [start])

  function submit(event: FormEvent) {
    event.preventDefault()
    enable.mutate(code, { onSuccess: (result) => onEnabled(result.recovery_codes) })
  }

  const setupError = fieldError(setup.error, 'code')
  return (
    <form onSubmit={submit} noValidate>
      <Card
        title={t('2FA instellen')}
        description={t('Scan de QR-code met een authenticator-app, zoals Google Authenticator, 1Password of Aegis.')}
        footer={
          <>
            <Button variant="ghost" onClick={onCancel}>
              {t('Annuleren')}
            </Button>
            <Button type="submit" variant="primary" disabled={!setup.data || enable.isPending || !code.trim()}>
              {t('Aanzetten')}
            </Button>
          </>
        }
      >
        {setupError ? (
          <p className="text-sm font-medium text-red-500 dark:text-red-400">{setupError}</p>
        ) : (
          <div className="flex flex-col gap-6 sm:flex-row">
            <div className="size-[196px] shrink-0 rounded-lg border border-zinc-200 bg-white p-2 dark:border-white/10">
              {setup.data && <div className="size-full [&_svg]:size-full" dangerouslySetInnerHTML={{ __html: setup.data.qr_svg }} />}
            </div>
            <div className="min-w-0 flex-1 space-y-6">
              <div>
                <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Kun je niet scannen?')}</p>
                <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('Vul deze sleutel dan met de hand in:')}</p>
                <code className="mt-2 block font-mono text-sm break-all text-zinc-800 dark:text-white">{setup.data?.secret.match(/.{1,4}/g)?.join(' ')}</code>
              </div>
              <Field label={t('Code uit de app')} error={fieldError(enable.error, 'code')}>
                {(id) => <Input id={id} inputMode="numeric" autoComplete="one-time-code" invalid={!!enable.error} value={code} onChange={(event) => setCode(event.target.value)} />}
              </Field>
            </div>
          </div>
        )}
      </Card>
    </form>
  )
}

function PasswordDialog({
  open,
  title,
  text,
  confirmLabel,
  danger,
  pending,
  error,
  onClose,
  onConfirm,
}: {
  open: boolean
  title: string
  text: string
  confirmLabel: string
  danger?: boolean
  pending: boolean
  error?: string
  codeError?: string
  onClose: () => void
  onConfirm: (input: { password: string; code: string }) => void
}) {
  const [password, setPassword] = useState('')
  const [code, setCode] = useState('')
  return (
    <ConfirmDialog
      open={open}
      title={title}
      confirmLabel={confirmLabel}
      danger={danger}
      pending={pending || !password || !code.trim()}
      onClose={() => {
        setPassword('')
        setCode('')
        onClose()
      }}
      onConfirm={() => onConfirm({ password, code })}
    >
      <p>{text}</p>
      <div className="mt-4 space-y-4">
        <Field label={t('Je wachtwoord')} error={codeError ? undefined : error}>
          {(id) => <Input id={id} type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} />}
        </Field>
        <Field label={t('Code uit de app of een herstelcode')} error={codeError}>
          {(id) => <Input id={id} autoComplete="one-time-code" value={code} onChange={(event) => setCode(event.target.value)} />}
        </Field>
      </div>
    </ConfirmDialog>
  )
}

export function TwoFactorTab() {
  const { data: status } = useTwoFactor()
  const disable = useDisableTwoFactor()
  const renew = useNewRecoveryCodes()
  const [mode, setMode] = useState<'idle' | 'setup' | 'disable' | 'renew'>('idle')
  const [codes, setCodes] = useState<string[] | null>(null)

  if (!status) return null
  if (codes) {
    return (
      <div className="max-w-xl">
        <RecoveryCodes codes={codes} onDone={() => setCodes(null)} />
      </div>
    )
  }
  if (mode === 'setup') {
    return (
      <div className="max-w-2xl">
        <Setup
          onCancel={() => setMode('idle')}
          onEnabled={(recovery) => {
            setMode('idle')
            setCodes(recovery)
          }}
        />
      </div>
    )
  }

  return (
    <div className="max-w-xl">
      <Card title={t('Tweestapsverificatie')} description={t('Een extra beveiligingslaag voor je account.')}>
        {status.enabled ? (
          <div className="space-y-4">
            <div className="flex items-start gap-3">
              <ShieldCheckIcon className="mt-0.5 size-5 shrink-0 text-green-600 dark:text-green-400" aria-hidden />
              <div className="text-sm">
                <p className="font-medium text-zinc-800 dark:text-white">{t('2FA staat aan sinds {date}', { date: formatDate(status.enabled_at ?? '') })}</p>
                <p className="mt-1 text-zinc-500 dark:text-white/70">{t('Je hebt nog {count} herstelcodes over.', { count: status.recovery_codes_left })}</p>
              </div>
            </div>
            <div className="flex flex-wrap justify-end gap-2">
              <Button size="sm" onClick={() => setMode('renew')}>
                {t('Nieuwe herstelcodes')}
              </Button>
              <Button size="sm" variant="danger" onClick={() => setMode('disable')}>
                {t('Uitzetten')}
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-between gap-6">
            <div className="text-sm">
              <p className="font-medium text-zinc-800 dark:text-white">{t('2FA instellen')}</p>
              <p className="mt-1 text-zinc-500 dark:text-white/70">{t('Bij het inloggen vragen we dan ook om een code uit je authenticator-app.')}</p>
            </div>
            <Button size="sm" variant="primary" onClick={() => setMode('setup')}>
              {t('Instellen')}
            </Button>
          </div>
        )}
      </Card>
      <PasswordDialog
        open={mode === 'disable'}
        title={t('2FA uitzetten?')}
        text={t('Daarna is je wachtwoord weer genoeg om in te loggen.')}
        confirmLabel={t('Uitzetten')}
        danger
        pending={disable.isPending}
        error={fieldError(disable.error, 'password')}
        codeError={codeError(disable.error)}
        onClose={() => setMode('idle')}
        onConfirm={(input) => disable.mutate(input, { onSuccess: () => setMode('idle') })}
      />
      <PasswordDialog
        open={mode === 'renew'}
        title={t('Nieuwe herstelcodes maken?')}
        text={t('Je oude herstelcodes werken daarna niet meer.')}
        confirmLabel={t('Nieuwe codes maken')}
        pending={renew.isPending}
        error={fieldError(renew.error, 'password')}
        codeError={codeError(renew.error)}
        onClose={() => setMode('idle')}
        onConfirm={(input) =>
          renew.mutate(input, {
            onSuccess: (result) => {
              setMode('idle')
              setCodes(result.recovery_codes)
            },
          })
        }
      />
    </div>
  )
}
