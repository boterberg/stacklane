import { ArrowRightIcon, PlayCircleIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { Field, Select } from '../../components/ui/Field'
import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { useMountIso, useServerIso } from '../../lib/isos'

export function ServerIso({ serverId }: { serverId: string }) {
  const { data, isPending } = useServerIso(serverId)
  const mount = useMountIso(serverId)
  const [chosen, setChosen] = useState('')
  const error = mount.error instanceof ApiError ? (Object.values(mount.error.fieldErrors)[0] ?? mount.error.message) : null
  if (isPending || !data) return null
  const pick = chosen || data.available[0]?.id || ''

  return (
    <div className="max-w-xl space-y-6">
      <Card title={t('Opstarten vanaf ISO')} description={t('Start deze server op vanaf een eigen ISO, bijvoorbeeld om zelf een besturingssysteem te installeren.')}>
        {data.mounted ? (
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <p className="flex items-center gap-2 text-sm font-medium text-zinc-800 dark:text-white">
                <PlayCircleIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {data.mounted.name}
              </p>
              <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{t('De server start op vanaf deze ISO. Gebruik de console om verder te gaan.')}</p>
            </div>
            <Button size="sm" icon={XMarkIcon} disabled={mount.isPending} onClick={() => mount.mutate(null)}>
              {t('Loskoppelen')}
            </Button>
          </div>
        ) : data.available.length === 0 ? (
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <p className="text-sm font-medium text-zinc-800 dark:text-white">{t("Geen ISO's beschikbaar")}</p>
              <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">{t('Importeer eerst een ISO en mount deze daarna op deze server.')}</p>
            </div>
            <ButtonLink size="sm" icon={ArrowRightIcon} to="/vps/isos">
              {t("Naar ISO's")}
            </ButtonLink>
          </div>
        ) : (
          <>
            <div className="flex flex-wrap items-end gap-3">
              <div className="min-w-0 flex-1">
                <Field label={t('ISO')}>
                  {(id) => (
                    <Select id={id} value={pick} onChange={(event) => setChosen(event.target.value)}>
                      {data.available.map((iso) => (
                        <option key={iso.id} value={iso.id}>
                          {iso.name}
                        </option>
                      ))}
                    </Select>
                  )}
                </Field>
              </div>
              <Button variant="primary" disabled={!pick || mount.isPending} onClick={() => mount.mutate(pick)}>
                {mount.isPending ? t('Bezig...') : t('Koppelen en herstarten')}
              </Button>
            </div>
            <p className="mt-3 text-sm text-zinc-500 dark:text-white/60">
              {t('De server start daarna met {firmware}, zoals de ISO vraagt. Dat blijft zo na het loskoppelen, zodat een geïnstalleerd systeem gewoon opstart.', {
                firmware: data.available.find((iso) => iso.id === pick)?.firmware === 'uefi' ? 'UEFI' : 'BIOS',
              })}
            </p>
          </>
        )}
        {error && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
      </Card>
    </div>
  )
}
