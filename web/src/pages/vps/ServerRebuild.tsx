import { ArrowPathIcon } from '@heroicons/react/24/outline'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'

import { Button } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { Checkbox, Field, Select } from '../../components/ui/Field'
import { api, ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { useSshKeys } from '../../lib/vps'

function errorText(error: unknown) {
  if (!(error instanceof ApiError)) return null
  return Object.values(error.fieldErrors)[0] ?? error.message
}

export function ServerRebuild({ serverId, rebuilding }: { serverId: string; rebuilding: boolean }) {
  const queryClient = useQueryClient()
  const { data: images = [] } = useQuery({
    queryKey: ['vps', 'images', serverId],
    queryFn: () => api<{ id: string; name: string }[]>(`/vps/servers/${serverId}/images`),
  })
  const { data: keys = [] } = useSshKeys()
  const [image, setImage] = useState('')
  const [chosenKeys, setChosenKeys] = useState<string[]>([])
  const [confirm, setConfirm] = useState(false)
  const rebuild = useMutation({
    mutationFn: () =>
      api<void>(`/vps/servers/${serverId}/rebuild`, {
        method: 'POST',
        body: { image: image || images[0]?.id, ssh_key_ids: chosenKeys },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['vps', 'server', serverId] }),
    onSettled: () => setConfirm(false),
  })
  const selected = images.find((item) => item.id === image) ?? images[0]

  return (
    <Card className="max-w-4xl" title={t('Server herbouwen')} description={t('Herbouw je server met de geselecteerde image. Alle bestaande gegevens op de schijf zullen permanent verloren gaan.')}>
      {rebuilding ? (
        <p className="text-sm text-zinc-500 dark:text-white/70">{t('Je server wordt herbouwd. Dit duurt meestal een paar minuten; daarna krijg je een nieuw rootwachtwoord.')}</p>
      ) : (
        <form
          noValidate
          className="space-y-6"
          onSubmit={(event) => {
            event.preventDefault()
            setConfirm(true)
          }}
        >
          <Field label={t('Besturingssysteem')}>
            {(id) => (
              <Select id={id} value={selected?.id ?? ''} onChange={(event) => setImage(event.target.value)}>
                {images.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t('SSH-sleutels')} description={keys.length === 0 ? t('Zonder SSH-sleutel log je in met het nieuwe rootwachtwoord.') : undefined}>
            {() => (
              <div className="space-y-2">
                {keys.map((key) => (
                  <Checkbox
                    key={key.id}
                    label={key.name}
                    checked={chosenKeys.includes(key.id)}
                    onChange={(event) => setChosenKeys(event.target.checked ? [...chosenKeys, key.id] : chosenKeys.filter((id) => id !== key.id))}
                  />
                ))}
              </div>
            )}
          </Field>
          {errorText(rebuild.error) && <p className="text-sm font-medium text-red-500 dark:text-red-400">{errorText(rebuild.error)}</p>}
          <div className="flex justify-end">
            <Button type="submit" variant="danger" icon={ArrowPathIcon} disabled={!selected || rebuild.isPending}>
              {t('Herbouwen')}
            </Button>
          </div>
        </form>
      )}
      <ConfirmDialog
        open={confirm}
        title={t('Server herbouwen?')}
        confirmLabel={t('Herbouwen')}
        danger
        pending={rebuild.isPending}
        onClose={() => setConfirm(false)}
        onConfirm={() => rebuild.mutate()}
      >
        {t('Alles op de schijf van deze server wordt gewist en {image} wordt opnieuw geïnstalleerd. Volumes, IP-adressen en netwerken blijven gekoppeld.', { image: selected?.name ?? '' })}
      </ConfirmDialog>
    </Card>
  )
}
