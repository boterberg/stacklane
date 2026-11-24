import { ArrowLeftIcon, DocumentIcon, FolderIcon } from '@heroicons/react/24/outline'
import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { Button } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { formatBytes, formatRelative } from '../../lib/format'
import { saveFile, useFileContent, useFiles } from '../../lib/gameservers'
import { t } from '../../lib/i18n'

const MAX_EDITABLE = 2 * 1024 * 1024

function join(directory: string, name: string) {
  return `${directory.replace(/\/$/, '')}/${name}`
}

function EditorForm({ serverId, file, initial, onClose }: { serverId: string; file: string; initial: string; onClose: () => void }) {
  const [content, setContent] = useState(initial)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)
  const queryClient = useQueryClient()

  async function save() {
    setSaving(true)
    setMessage(null)
    try {
      await saveFile(serverId, file, content)
      await queryClient.invalidateQueries({ queryKey: ['gameserver', serverId, 'file', file] })
      setMessage(t('Opgeslagen'))
    } catch (failure) {
      setMessage(failure instanceof Error ? failure.message : t('Opslaan mislukt.'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div>
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <button type="button" onClick={onClose} className="flex items-center gap-2 text-sm font-medium text-zinc-500 hover:text-zinc-800 dark:text-white/70 dark:hover:text-white">
          <ArrowLeftIcon className="size-4" />
          {t('Terug naar de map')}
        </button>
        <div className="flex items-center gap-3">
          {message && <span className="text-sm text-zinc-500 dark:text-white/70">{message}</span>}
          <Button size="sm" variant="primary" onClick={save} disabled={saving}>
            {t('Opslaan')}
          </Button>
        </div>
      </div>
      <p className="mb-2 font-mono text-sm text-zinc-800 dark:text-white">{file}</p>
      <textarea
        value={content}
        onChange={(event) => setContent(event.target.value)}
        spellCheck={false}
        className="h-[480px] w-full rounded-xl border border-zinc-200 bg-[#18181b] p-4 font-mono text-[13px] leading-6 text-zinc-100 focus:outline-2 focus:outline-zinc-800/15 dark:border-white/10"
      />
    </div>
  )
}

function Editor({ serverId, file, onClose }: { serverId: string; file: string; onClose: () => void }) {
  const { data, error } = useFileContent(serverId, file)
  if (error) return <EmptyState title={t('Bestand niet te openen')} description={error.message} />
  if (!data) return null
  return <EditorForm key={file} serverId={serverId} file={file} initial={data.content} onClose={onClose} />
}

export function ServerFiles({ serverId }: { serverId: string }) {
  const [directory, setDirectory] = useState('/')
  const [file, setFile] = useState<string | null>(null)
  const { data, isPending, error } = useFiles(serverId, directory)

  if (file) {
    return <Editor serverId={serverId} file={file} onClose={() => setFile(null)} />
  }

  const parts = directory.split('/').filter(Boolean)
  const entries = [...(data?.entries ?? [])].sort((left, right) => Number(left.is_file) - Number(right.is_file) || left.name.localeCompare(right.name))

  return (
    <div>
      <nav className="mb-4 flex flex-wrap items-center gap-1 font-mono text-sm" aria-label={t('Pad')}>
        <button type="button" onClick={() => setDirectory('/')} className="rounded px-1.5 py-0.5 text-zinc-500 hover:bg-zinc-800/5 hover:text-zinc-800 dark:text-white/70 dark:hover:bg-white/10 dark:hover:text-white">
          /
        </button>
        {parts.map((part, index) => (
          <span key={index} className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setDirectory(`/${parts.slice(0, index + 1).join('/')}`)}
              className="rounded px-1.5 py-0.5 text-zinc-500 hover:bg-zinc-800/5 hover:text-zinc-800 dark:text-white/70 dark:hover:bg-white/10 dark:hover:text-white"
            >
              {part}
            </button>
            {index < parts.length - 1 && <span className="text-zinc-400">/</span>}
          </span>
        ))}
      </nav>

      {error ? (
        <EmptyState title={t('Bestanden niet beschikbaar')} description={error.message} />
      ) : !isPending && entries.length === 0 ? (
        <EmptyState title={t('Lege map')} description={t('Hier staan nog geen bestanden.')} />
      ) : (
        <div className="overflow-hidden rounded-xl border border-zinc-200 dark:border-white/10">
          <table className="w-full text-left text-sm">
            <thead className="bg-zinc-50 dark:bg-white/5">
              <tr>
                <th className="px-4 py-2.5 font-medium text-zinc-800 dark:text-white">{t('Naam')}</th>
                <th className="hidden px-4 py-2.5 font-medium text-zinc-800 sm:table-cell dark:text-white">{t('Grootte')}</th>
                <th className="hidden px-4 py-2.5 font-medium text-zinc-800 md:table-cell dark:text-white">{t('Gewijzigd')}</th>
              </tr>
            </thead>
            <tbody>
              {directory !== '/' && (
                <tr className="border-t border-zinc-200 dark:border-white/10">
                  <td colSpan={3} className="px-4 py-2.5">
                    <button type="button" onClick={() => setDirectory(`/${parts.slice(0, -1).join('/')}`)} className="flex items-center gap-3 text-zinc-500 hover:text-zinc-800 dark:text-white/70 dark:hover:text-white">
                      <FolderIcon className="size-4" />
                      ..
                    </button>
                  </td>
                </tr>
              )}
              {entries.map((entry) => {
                const editable = entry.is_file && entry.size <= MAX_EDITABLE
                const Icon = entry.is_file ? DocumentIcon : FolderIcon
                return (
                  <tr key={entry.name} className="border-t border-zinc-200 dark:border-white/10">
                    <td className="px-4 py-2.5">
                      <button
                        type="button"
                        disabled={entry.is_file && !editable}
                        onClick={() => (entry.is_file ? setFile(join(directory, entry.name)) : setDirectory(join(directory, entry.name)))}
                        className="flex items-center gap-3 font-medium text-zinc-800 hover:underline disabled:no-underline disabled:opacity-60 dark:text-white"
                      >
                        <Icon className={entry.is_file ? 'size-4 text-zinc-400' : 'size-4 text-indigo-500 dark:text-indigo-300'} />
                        {entry.name}
                      </button>
                    </td>
                    <td className="hidden px-4 py-2.5 text-zinc-500 sm:table-cell dark:text-white/70">{entry.is_file ? formatBytes(entry.size) : ''}</td>
                    <td className="hidden px-4 py-2.5 text-zinc-500 md:table-cell dark:text-white/70">{formatRelative(entry.modified_at)}</td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
