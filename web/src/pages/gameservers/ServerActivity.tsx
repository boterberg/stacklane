import { ArchiveBoxArrowDownIcon, ArchiveBoxIcon, CommandLineIcon, DocumentTextIcon, PowerIcon, PuzzlePieceIcon } from '@heroicons/react/24/outline'

import { EmptyState } from '../../components/ui/EmptyState'
import { formatDate, formatRelative } from '../../lib/format'
import { useActivity, type ActivityEntry } from '../../lib/gameservers'
import { intlLocale, t } from '../../lib/i18n'

function powerLabel(signal: string | null) {
  switch (signal) {
    case 'start':
      return t('Server gestart')
    case 'stop':
      return t('Server gestopt')
    case 'restart':
      return t('Server herstart')
    case 'kill':
      return t('Server geforceerd gestopt')
    default:
      return t('Server aan- of uitgezet')
  }
}

function describe(entry: ActivityEntry) {
  switch (entry.action) {
    case 'power':
      return { icon: PowerIcon, text: powerLabel(entry.detail) }
    case 'console':
      return { icon: CommandLineIcon, text: t('Console geopend') }
    case 'file.write':
      return { icon: DocumentTextIcon, text: t('Bestand opgeslagen: {file}', { file: entry.detail ?? '' }) }
    case 'backup.create':
      return { icon: ArchiveBoxIcon, text: t('Back-up gemaakt: {name}', { name: entry.detail ?? '' }) }
    case 'backup.download':
      return { icon: ArchiveBoxArrowDownIcon, text: t('Back-up gedownload') }
    case 'addon.install':
      return { icon: PuzzlePieceIcon, text: t('Geïnstalleerd: {name}', { name: entry.detail ?? '' }) }
    case 'addon.remove':
      return { icon: PuzzlePieceIcon, text: t('Verwijderd: {name}', { name: entry.detail ?? '' }) }
    default:
      return { icon: DocumentTextIcon, text: entry.action }
  }
}

export function ServerActivity({ serverId }: { serverId: string }) {
  const { data: entries, isPending } = useActivity(serverId)
  if (!isPending && (entries ?? []).length === 0) {
    return <EmptyState title={t('Nog geen activiteit')} description={t('Hier zie je wie de server aan- of uitzette, bestanden wijzigde en back-ups maakte.')} />
  }
  return (
    <ul className="divide-y divide-zinc-200 rounded-xl border border-zinc-200 bg-white dark:divide-white/10 dark:border-white/10 dark:bg-white/5">
      {(entries ?? []).map((entry) => {
        const { icon: Icon, text } = describe(entry)
        const time = new Intl.DateTimeFormat(intlLocale(), { hour: '2-digit', minute: '2-digit' }).format(new Date(entry.created_at))
        return (
          <li key={entry.id} className="flex items-start gap-3 px-5 py-3">
            <Icon className="mt-0.5 size-5 shrink-0 text-zinc-400 dark:text-white/50" aria-hidden />
            <div className="min-w-0 flex-1">
              <p className="text-sm break-words text-zinc-800 dark:text-white">{text}</p>
              <p className="mt-0.5 text-xs text-zinc-500 dark:text-white/60">{[entry.actor ?? t('Onbekend'), entry.ip].filter(Boolean).join(' · ')}</p>
            </div>
            <time dateTime={entry.created_at} title={`${formatDate(entry.created_at)} ${time}`} className="shrink-0 text-xs text-zinc-500 dark:text-white/60">
              {formatRelative(entry.created_at)}
            </time>
          </li>
        )
      })}
    </ul>
  )
}
