import { KeyIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { SlideOver } from '../../components/layout/SlideOver'
import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { Field, Input } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ApiError } from '../../lib/api'
import { formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useAddSshKey, useDeleteSshKey, useSshKeys, type SshKey } from '../../lib/vps'

function AddKey({ open, onClose }: { open: boolean; onClose: () => void }) {
  const add = useAddSshKey()
  const [name, setName] = useState('')
  const [publicKey, setPublicKey] = useState('')
  const errors = add.error instanceof ApiError ? add.error.fieldErrors : {}
  function close() {
    setName('')
    setPublicKey('')
    add.reset()
    onClose()
  }
  return (
    <SlideOver
      open={open}
      onClose={close}
      title={t('SSH-sleutel toevoegen')}
      description={t('Nieuwe servers krijgen de sleutels die je bij het bestellen aanvinkt.')}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t('Annuleren')}
          </Button>
          <Button variant="primary" disabled={add.isPending} onClick={() => add.mutate({ name, public_key: publicKey }, { onSuccess: close })}>
            {t('Toevoegen')}
          </Button>
        </>
      }
    >
      <div className="space-y-6">
        <Field label={t('Naam')} error={errors.name}>
          {(id) => <Input id={id} placeholder={t('Bijvoorbeeld Laptop')} invalid={!!errors.name} value={name} onChange={(event) => setName(event.target.value)} />}
        </Field>
        <Field label={t('Publieke sleutel')} description={t('De inhoud van je .pub-bestand, bijvoorbeeld ~/.ssh/id_ed25519.pub.')} error={errors.public_key}>
          {(id) => (
            <textarea
              id={id}
              rows={5}
              spellCheck={false}
              value={publicKey}
              onChange={(event) => setPublicKey(event.target.value)}
              placeholder="ssh-ed25519 AAAA..."
              className="block w-full rounded-lg border border-zinc-200 bg-white px-3 py-2 font-mono text-xs text-zinc-700 shadow-xs focus:outline-2 focus:-outline-offset-1 focus:outline-zinc-800/15 dark:border-white/10 dark:bg-white/10 dark:text-zinc-300"
            />
          )}
        </Field>
      </div>
    </SlideOver>
  )
}

export function SshKeysPage() {
  const { data: keys, isPending } = useSshKeys()
  const remove = useDeleteSshKey()
  const [adding, setAdding] = useState(false)
  const [deleting, setDeleting] = useState<SshKey | null>(null)

  return (
    <>
      <PageHeader
        title={t('SSH-sleutels')}
        actions={
          <Button size="sm" icon={PlusIcon} onClick={() => setAdding(true)}>
            {t('Sleutel toevoegen')}
          </Button>
        }
      />
      {!isPending && (keys ?? []).length === 0 ? (
        <EmptyState title={t('Geen SSH-sleutels')} description={t('Voeg een sleutel toe om zonder wachtwoord in te loggen op je servers.')} />
      ) : (
        <ul className="space-y-3">
          {(keys ?? []).map((key) => (
            <li key={key.id} className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-5 dark:border-white/10 dark:bg-white/10">
              <div className="flex min-w-0 items-start gap-3">
                <KeyIcon className="mt-0.5 size-5 shrink-0 text-zinc-400 dark:text-white/50" aria-hidden />
                <div className="min-w-0">
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{key.name}</p>
                  <p className="mt-1 truncate font-mono text-xs text-zinc-500 dark:text-white/60">{key.fingerprint}</p>
                  <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{t('Toegevoegd {when}', { when: formatRelative(key.created_at) })}</p>
                </div>
              </div>
              <Button size="sm" icon={TrashIcon} onClick={() => setDeleting(key)}>
                {t('Verwijderen')}
              </Button>
            </li>
          ))}
        </ul>
      )}
      <AddKey open={adding} onClose={() => setAdding(false)} />
      <ConfirmDialog
        open={deleting !== null}
        title={t('Sleutel verwijderen?')}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setDeleting(null)}
        onConfirm={() => deleting && remove.mutate(deleting.id, { onSettled: () => setDeleting(null) })}
      >
        {t('Servers die al draaien houden de sleutel. Nieuwe servers krijgen hem niet meer.')}
      </ConfirmDialog>
    </>
  )
}
