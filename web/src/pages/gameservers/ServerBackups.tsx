import { ArrowDownTrayIcon, PlusIcon } from '@heroicons/react/24/outline'
import { useState, type FormEvent } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { Input } from '../../components/ui/Field'
import { formatBytes, formatDate, formatRelative } from '../../lib/format'
import { backupDownloadUrl, useBackups, useCreateBackup } from '../../lib/gameservers'
import { t } from '../../lib/i18n'

export function ServerBackups({ serverId }: { serverId: string }) {
  const { data: backups, isPending, error } = useBackups(serverId)
  const create = useCreateBackup(serverId)
  const [name, setName] = useState('')

  function submit(event: FormEvent) {
    event.preventDefault()
    create.mutate(name.trim() || t('Back-up {date}', { date: formatDate(new Date().toISOString()) }), { onSuccess: () => setName('') })
  }

  async function download(backup: string) {
    window.location.href = await backupDownloadUrl(serverId, backup)
  }

  return (
    <div className="space-y-6">
      <form onSubmit={submit} className="flex flex-wrap items-center gap-3">
        <Input value={name} onChange={(event) => setName(event.target.value)} placeholder={t('Naam, bijvoorbeeld Voor de update')} maxLength={60} className="max-w-sm" />
        <Button type="submit" icon={PlusIcon} disabled={create.isPending}>
          {t('Back-up maken')}
        </Button>
        {create.error && <p className="text-sm text-red-500">{create.error.message}</p>}
      </form>

      {error ? (
        <EmptyState title={t('Back-ups niet beschikbaar')} description={error.message} />
      ) : !isPending && (backups ?? []).length === 0 ? (
        <EmptyState title={t('Nog geen back-ups')} description={t('Maak een back-up voordat je iets groots aan je server verandert.')} />
      ) : (
        <ul className="space-y-3">
          {[...(backups ?? [])].reverse().map((backup) => (
            <li key={backup.uuid} className="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-zinc-200 bg-white px-6 py-4 dark:border-white/10 dark:bg-white/10">
              <div>
                <div className="flex items-center gap-3">
                  <p className="text-sm font-medium text-zinc-800 dark:text-white">{backup.name}</p>
                  {!backup.completed_at ? <Badge color="sky">{t('Bezig')}</Badge> : backup.is_successful ? null : <Badge color="red">{t('Mislukt')}</Badge>}
                </div>
                <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">
                  {formatRelative(backup.created_at)}
                  {backup.completed_at && backup.is_successful && <> · {formatBytes(backup.bytes)}</>}
                </p>
              </div>
              {backup.completed_at && backup.is_successful && (
                <Button size="sm" icon={ArrowDownTrayIcon} onClick={() => download(backup.uuid)}>
                  {t('Downloaden')}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
