import { LifebuoyIcon } from '@heroicons/react/24/outline'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { api, ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

export function ServerRescue({ serverId, ipv4, active }: { serverId: string; ipv4?: string; active: boolean }) {
  const queryClient = useQueryClient()
  const [confirm, setConfirm] = useState(false)
  const [password, setPassword] = useState<string | null>(null)
  const refresh = () => queryClient.invalidateQueries({ queryKey: ['vps', 'server', serverId] })
  const enter = useMutation({
    mutationFn: () => api<{ password: string }>(`/vps/servers/${serverId}/rescue`, { method: 'POST' }),
    onSuccess: (result) => {
      setPassword(result.password)
      refresh()
    },
    onSettled: () => setConfirm(false),
  })
  const leave = useMutation({
    mutationFn: () => api<void>(`/vps/servers/${serverId}/rescue`, { method: 'DELETE' }),
    onSuccess: () => {
      setPassword(null)
      refresh()
    },
  })
  const error = errorText(enter.error ?? leave.error)

  return (
    <Card
      className="max-w-4xl"
      title={t('Rescue mode')}
      description={t('Start deze server op in een tijdelijke herstelomgeving, bijvoorbeeld om een kapotte configuratie te repareren of bestanden terug te halen. Je schijf blijft gewoon gekoppeld.')}
    >
      {active ? (
        <div className="space-y-4 text-sm text-zinc-600 dark:text-white/70">
          <p>{t('Je server draait nu in rescue-modus. Log in met SSH op hetzelfde IP-adres; je eigen schijf staat als tweede schijf klaar om te mounten.')}</p>
          {ipv4 && <code className="block font-mono text-zinc-800 dark:text-white">ssh root@{ipv4}</code>}
          {password ? (
            <div>
              <p className="font-medium text-zinc-800 dark:text-white">{t('Tijdelijk wachtwoord')}</p>
              <code className="mt-1 block font-mono text-zinc-800 dark:text-white">{password}</code>
              <p className="mt-1">{t('Je ziet dit wachtwoord maar één keer. Het werkt alleen zolang rescue-modus aan staat.')}</p>
            </div>
          ) : (
            <p>{t('Ben je het tijdelijke wachtwoord kwijt? Zet rescue-modus uit en weer aan voor een nieuw wachtwoord.')}</p>
          )}
          <div className="flex justify-end">
            <Button icon={LifebuoyIcon} disabled={leave.isPending} onClick={() => leave.mutate()}>
              {leave.isPending ? t('Bezig...') : t('Uitschakelen')}
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Rescue mode inschakelen')}</p>
            <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('De server herstart in de herstelomgeving. Je krijgt een tijdelijk wachtwoord om met SSH in te loggen.')}</p>
          </div>
          <Button variant="danger" icon={LifebuoyIcon} disabled={enter.isPending} onClick={() => setConfirm(true)}>
            {t('Inschakelen')}
          </Button>
        </div>
      )}
      {error && <p className="mt-4 text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
      <ConfirmDialog
        open={confirm}
        title={t('Rescue mode inschakelen?')}
        confirmLabel={t('Inschakelen')}
        danger
        pending={enter.isPending}
        onClose={() => setConfirm(false)}
        onConfirm={() => enter.mutate()}
      >
        {t('De server wordt meteen herstart. Diensten op de server zijn onbereikbaar tot je rescue-modus weer uitzet.')}
      </ConfirmDialog>
    </Card>
  )
}
