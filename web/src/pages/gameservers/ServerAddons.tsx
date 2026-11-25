import { ArrowDownTrayIcon, CheckIcon, PuzzlePieceIcon, TrashIcon } from '@heroicons/react/24/outline'
import { useEffect, useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Field, FormError, Input, Select } from '../../components/ui/Field'
import { SearchField } from '../../components/ui/ListToolbar'
import { formatBytes } from '../../lib/format'
import { loaderName, useAddonChanges, useAddonContext, useAddonSearch, useInstalledAddons, usesPlugins, type AddonContext, type AddonHit } from '../../lib/gameservers'
import { intlLocale, t } from '../../lib/i18n'

function Installed({ serverId, loader }: { serverId: string; loader: string }) {
  const { data, error } = useInstalledAddons(serverId, loader)
  const { remove } = useAddonChanges(serverId)
  const [confirm, setConfirm] = useState<string | null>(null)
  const files = data?.files ?? []
  return (
    <Card title={t('Geïnstalleerd')} description={data ? t('Bestanden in {folder}.', { folder: data.folder }) : undefined}>
      {error && <p className="text-sm text-zinc-500 dark:text-white/70">{t('We kunnen de bestanden van de server nu niet ophalen.')}</p>}
      {data && files.length === 0 && <p className="text-sm text-zinc-500 dark:text-white/70">{usesPlugins(loader) ? t('Nog geen plugins.') : t('Nog geen mods.')}</p>}
      {files.length > 0 && (
        <ul className="divide-y divide-zinc-200 dark:divide-white/10">
          {files.map((file) => (
            <li key={file.name} className="flex items-center gap-3 py-2.5 first:pt-0 last:pb-0">
              <PuzzlePieceIcon className="size-5 shrink-0 text-zinc-400" aria-hidden />
              <span className="min-w-0 flex-1 truncate font-mono text-sm text-zinc-800 dark:text-white">{file.name}</span>
              <span className="shrink-0 text-sm text-zinc-500 tabular-nums dark:text-white/60">{formatBytes(file.size)}</span>
              <Button size="sm" variant="ghost" icon={TrashIcon} onClick={() => setConfirm(file.name)}>
                <span className="sr-only sm:not-sr-only">{t('Verwijderen')}</span>
              </Button>
            </li>
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={confirm !== null}
        title={t('{file} verwijderen?', { file: confirm ?? '' })}
        confirmLabel={t('Verwijderen')}
        danger
        pending={remove.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() => confirm && remove.mutate({ loader, file: confirm }, { onSettled: () => setConfirm(null) })}
      >
        {t('Het bestand verdwijnt van de server. Instellingen die de plugin in een eigen map bewaart, blijven staan. Herstart de server daarna.')}
      </ConfirmDialog>
    </Card>
  )
}

function Result({ hit, onInstall, installing, installed }: { hit: AddonHit; onInstall: () => void; installing: boolean; installed: boolean }) {
  return (
    <li className="flex gap-4 rounded-xl border border-zinc-200 bg-white p-4 dark:border-white/10 dark:bg-white/5">
      {hit.icon_url ? (
        <img src={hit.icon_url} alt="" loading="lazy" referrerPolicy="no-referrer" className="size-12 shrink-0 rounded-lg bg-zinc-100 object-cover dark:bg-white/10" />
      ) : (
        <span className="flex size-12 shrink-0 items-center justify-center rounded-lg bg-zinc-100 dark:bg-white/10">
          <PuzzlePieceIcon className="size-6 text-zinc-400" aria-hidden />
        </span>
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-zinc-800 dark:text-white">{hit.title}</p>
        <p className="truncate text-xs text-zinc-500 dark:text-white/60">
          {t('door {author}', { author: hit.author })} · {t('{count} downloads', { count: new Intl.NumberFormat(intlLocale(), { notation: 'compact' }).format(hit.downloads) })}
        </p>
        <p className="mt-1 line-clamp-2 text-sm text-zinc-500 dark:text-white/70">{hit.description}</p>
      </div>
      <div className="shrink-0 self-center">
        {installed ? (
          <Badge color="green">
            <CheckIcon className="mr-1 size-4" aria-hidden />
            {t('Geïnstalleerd')}
          </Badge>
        ) : (
          <Button size="sm" icon={ArrowDownTrayIcon} disabled={installing} onClick={onInstall}>
            {t('Installeren')}
          </Button>
        )}
      </div>
    </li>
  )
}

function Manager({ serverId, context }: { serverId: string; context: AddonContext }) {
  const [loader, setLoader] = useState(context.loader ?? 'paper')
  const [version, setVersion] = useState(context.version ?? '')
  const [query, setQuery] = useState('')
  const [debounced, setDebounced] = useState('')
  const [done, setDone] = useState<string[]>([])
  const { install } = useAddonChanges(serverId)
  const validVersion = /^\d{1,3}\.\d{1,3}(\.\d{1,3})?$/.test(version.trim()) ? version.trim() : ''
  const { data: hits, isFetching, error } = useAddonSearch(serverId, loader, debounced, validVersion)

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query.trim()), 350)
    return () => clearTimeout(timer)
  }, [query])

  return (
    <div className="space-y-6">
      {!context.loader && (
        <p className="rounded-lg bg-amber-400/15 px-4 py-3 text-sm text-amber-800 dark:text-amber-200">
          {t('We herkennen het servertype niet aan de instellingen van deze server. Kies het hieronder zelf.')}
        </p>
      )}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <Field label={t('Servertype')} description={usesPlugins(loader) ? t('Plugins komen in /plugins.') : t('Mods komen in /mods.')}>
          {(id) => (
            <Select id={id} value={loader} onChange={(event) => setLoader(event.target.value)}>
              {context.loaders.map((option) => (
                <option key={option} value={option}>
                  {loaderName(option)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('Minecraft-versie')} description={t('We tonen alleen wat op deze versie werkt. Leeg laten toont alles.')}>
          {(id) => <Input id={id} placeholder="1.21.1" value={version} onChange={(event) => setVersion(event.target.value)} />}
        </Field>
      </div>

      <Installed serverId={serverId} loader={loader} />

      <section>
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-base font-medium text-zinc-800 dark:text-white">{usesPlugins(loader) ? t('Plugins zoeken') : t('Mods zoeken')}</h2>
          <div className="w-full sm:w-72">
            <SearchField value={query} onChange={setQuery} placeholder={t('Zoek op Modrinth...')} />
          </div>
        </div>
        <FormError error={install.error ?? error} />
        {hits && hits.length === 0 && !isFetching && <p className="text-sm text-zinc-500 dark:text-white/70">{t('Niets gevonden voor dit servertype en deze versie.')}</p>}
        {hits && hits.length > 0 && (
          <ul className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {hits.map((hit) => (
              <Result
                key={hit.project_id}
                hit={hit}
                installed={done.includes(hit.project_id)}
                installing={install.isPending && install.variables?.project === hit.project_id}
                onInstall={() => install.mutate({ project: hit.project_id, loader, version: validVersion }, { onSuccess: () => setDone((list) => [...list, hit.project_id]) })}
              />
            ))}
          </ul>
        )}
        <p className="mt-4 text-sm text-zinc-500 dark:text-white/70">{t('Herstart de server nadat je iets hebt geïnstalleerd of verwijderd. Plugins en mods komen van Modrinth; Stacklane controleert ze niet.')}</p>
      </section>
    </div>
  )
}

export function ServerAddons({ serverId }: { serverId: string }) {
  const { data: context, error } = useAddonContext(serverId)
  if (error) return <FormError error={error} />
  if (!context) return null
  return <Manager serverId={serverId} context={context} />
}
