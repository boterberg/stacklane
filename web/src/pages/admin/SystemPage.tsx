import { ArrowPathIcon } from '@heroicons/react/24/outline'

import { UpdatePanel } from '../../components/admin/UpdatePanel'
import { Button } from '../../components/ui/Button'
import { Card, CardText } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/Heading'
import { formatRelative } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useMeta } from '../../lib/meta'
import { needsAttention, useCheckForUpdate, useUpdateStatus } from '../../lib/system'

export function SystemPage() {
  const { data: overview } = useUpdateStatus(true)
  const { data: meta } = useMeta()
  const check = useCheckForUpdate()

  return (
    <>
      <PageHeader title={t('Systeem')} />
      <Card
        title={t('Versie')}
        description={
          overview?.checked_at ? t('Laatst gecontroleerd {when}.', { when: formatRelative(overview.checked_at) }) : t('Nog niet gecontroleerd op updates.')
        }
        action={
          <Button size="sm" icon={ArrowPathIcon} disabled={check.isPending} onClick={() => check.mutate()}>
            {t('Nu controleren')}
          </Button>
        }
      >
        {overview ? (
          <div className="space-y-4">
            <CardText>{t('Je draait Stacklane {version}.', { version: overview.current })}</CardText>
            {needsAttention(overview) || overview.progress?.state === 'failed' ? (
              <UpdatePanel overview={overview} demo={!!meta?.demo} />
            ) : (
              <CardText>{t('Dit is de nieuwste versie.')}</CardText>
            )}
            {check.isError && <p className="text-sm font-medium text-red-500 dark:text-red-400">{t('De pakketbron is nu niet bereikbaar. Probeer het later nog eens.')}</p>}
          </div>
        ) : (
          <CardText>{t('Laden...')}</CardText>
        )}
      </Card>
    </>
  )
}
