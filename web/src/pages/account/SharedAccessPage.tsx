import { ArrowRightIcon, CheckIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { useNavigate } from 'react-router'

import { Button } from '../../components/ui/Button'
import { ConfirmDialog } from '../../components/ui/ConfirmDialog'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { DataTable } from '../../components/ui/ResourceList'
import { currentVpsOwner } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useAcceptInvitation, useLeaveEnvironment, useSharedEnvironments, useSwitchEnvironment, type SharedEnvironment } from '../../lib/vpsAccess'

export function SharedAccessPage() {
  const { data, isPending } = useSharedEnvironments()
  const leave = useLeaveEnvironment()
  const accept = useAcceptInvitation()
  const switchTo = useSwitchEnvironment()
  const navigate = useNavigate()
  const [confirm, setConfirm] = useState<SharedEnvironment | null>(null)
  const active = currentVpsOwner()
  const shared = data ?? []

  function open(owner: string | null) {
    switchTo(owner)
    navigate('/vps/servers')
  }

  return (
    <>
      <PageHeader
        title={t('Gedeelde toegang')}
        description={t('VPS-omgevingen van anderen waar jij toegang toe hebt.')}
        actions={
          active ? (
            <Button size="sm" onClick={() => open(null)}>
              {t('Naar je eigen omgeving')}
            </Button>
          ) : undefined
        }
      />
      {!isPending && shared.length === 0 ? (
        <EmptyState title={t('Geen gedeelde toegang')} description={t('Niemand heeft je nog toegang gegeven tot een VPS-omgeving.')} />
      ) : (
        <DataTable<SharedEnvironment>
          rows={shared}
          rowKey={(item) => item.owner_id}
          columns={[
            {
              header: t('Omgeving van'),
              cell: (item) => (
                <div>
                  <p className="font-medium text-zinc-800 dark:text-white">{item.name}</p>
                  <p className="text-zinc-500 dark:text-white/60">{item.email}</p>
                </div>
              ),
            },
            { header: t('Rechten'), cell: (item) => item.permissions.length },
            { header: t('Sinds'), cell: (item) => (item.accepted ? formatDate(item.created_at) : t('Uitnodiging')) },
            {
              header: '',
              className: 'text-right',
              cell: (item) =>
                item.accepted ? (
                  <div className="flex justify-end gap-2">
                    <Button size="sm" icon={ArrowRightIcon} variant={active === item.owner_id ? 'primary' : undefined} onClick={() => open(item.owner_id)}>
                      {active === item.owner_id ? t('Geopend') : t('Openen')}
                    </Button>
                    <Button size="sm" icon={XMarkIcon} onClick={() => setConfirm(item)}>
                      {t('Toegang opgeven')}
                    </Button>
                  </div>
                ) : (
                  <div className="flex justify-end gap-2">
                    <Button size="sm" icon={CheckIcon} variant="primary" disabled={accept.isPending} onClick={() => accept.mutate(item.owner_id)}>
                      {t('Accepteren')}
                    </Button>
                    <Button size="sm" icon={XMarkIcon} disabled={leave.isPending} onClick={() => leave.mutate(item.owner_id)}>
                      {t('Weigeren')}
                    </Button>
                  </div>
                ),
            },
          ]}
        />
      )}
      <ConfirmDialog
        open={confirm !== null}
        title={t('Toegang opgeven?')}
        confirmLabel={t('Opgeven')}
        danger
        pending={leave.isPending}
        onClose={() => setConfirm(null)}
        onConfirm={() =>
          confirm &&
          leave.mutate(confirm.owner_id, {
            onSuccess: () => {
              if (active === confirm.owner_id) switchTo(null)
            },
            onSettled: () => setConfirm(null),
          })
        }
      >
        {t('Je kunt daarna niet meer bij de VPS-omgeving van {name}, tot die je opnieuw toegang geeft.', { name: confirm?.name ?? '' })}
      </ConfirmDialog>
    </>
  )
}
