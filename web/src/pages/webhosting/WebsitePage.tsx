import { ArrowTopRightOnSquareIcon, CalendarIcon, CubeIcon } from '@heroicons/react/24/outline'
import { Link, useParams } from 'react-router'

import { Badge } from '../../components/ui/Badge'
import { Button, ButtonLink } from '../../components/ui/Button'
import { Card } from '../../components/ui/Card'
import { EmptyState } from '../../components/ui/EmptyState'
import { ApiError } from '../../lib/api'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { serviceStatusColors, serviceStatusLabels } from '../../lib/labels'
import { usePanelLogin, useWebsite } from '../../lib/webhosting'

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="py-3 first:pt-0 last:pb-0">
      <dt className="text-sm font-medium text-zinc-800 dark:text-white">{label}</dt>
      <dd className="mt-1 text-sm text-zinc-500 dark:text-white/70">{children}</dd>
    </div>
  )
}

export function WebsitePage() {
  const { id = '' } = useParams()
  const { data: website, isPending, error } = useWebsite(id)
  const login = usePanelLogin(id)

  if (isPending) return null
  if (error || !website) {
    return (
      <EmptyState
        title={t('Pakket niet gevonden')}
        description={t('Dit webhostingpakket bestaat niet of is van iemand anders.')}
        action={
          <ButtonLink to="/webhosting" size="sm">
            {t('Naar webhosting')}
          </ButtonLink>
        }
      />
    )
  }

  const panel = website.details.panel ?? t('het hostingpanel')
  const openPanel = () => {
    const tab = window.open('', '_blank')
    login.mutate(undefined, {
      onSuccess: ({ url }) => {
        if (tab) tab.location.href = url
        else window.location.href = url
      },
      onError: () => tab?.close(),
    })
  }

  return (
    <>
      <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="text-xl font-medium text-zinc-800 dark:text-white">{website.name}</h1>
            <Badge color={serviceStatusColors[website.status]}>{serviceStatusLabels[website.status]}</Badge>
          </div>
          <div className="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 text-sm text-zinc-500 dark:text-white/70">
            {website.product_name && (
              <span className="flex items-center gap-2">
                <CubeIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {website.product_name}
              </span>
            )}
            {website.expires_at && (
              <span className="flex items-center gap-2">
                <CalendarIcon className="size-4 text-zinc-400 dark:text-white/40" aria-hidden />
                {t('Verloopt op {date}', { date: formatDate(website.expires_at) })}
              </span>
            )}
          </div>
        </div>
        {website.status === 'active' && (
          <Button variant="primary" icon={ArrowTopRightOnSquareIcon} disabled={login.isPending} onClick={openPanel}>
            {t('Inloggen op {panel}', { panel })}
          </Button>
        )}
      </div>
      {login.error instanceof ApiError && <p className="mb-6 text-sm font-medium text-red-500 dark:text-red-400">{login.error.fieldErrors.website ?? login.error.message}</p>}

      {website.status !== 'active' ? (
        <EmptyState
          title={website.status === 'failed' ? t('Er ging iets mis') : website.status === 'suspended' ? t('Dit pakket is opgeschort') : t('Je webhosting wordt klaargezet')}
          description={
            website.status === 'failed'
              ? t('We konden je webhosting niet aanmaken. Ons team is op de hoogte.')
              : website.status === 'suspended'
                ? t('Betaal de openstaande factuur om je website weer online te zetten.')
                : website.status === 'pending'
                  ? t('Zodra de factuur betaald is, gaan we aan de slag.')
                  : t('Dit duurt meestal een paar minuten.')
          }
        />
      ) : (
        <div className="grid max-w-5xl gap-6 lg:grid-cols-2">
          <Card title={t('Pakket')}>
            <dl className="divide-y divide-zinc-200 dark:divide-white/10">
              <Row label={t('Domein')}>{website.name}</Row>
              <Row label={t('Gebruikersnaam')}>
                <span className="font-mono">{website.details.username ?? '-'}</span>
              </Row>
              <Row label={t('Hostingpanel')}>{website.details.panel ?? '-'}</Row>
              <Row label={t('Abonnement')}>
                {website.subscription_code ? (
                  <Link to={`/billing/subscriptions/${website.subscription_code}`} className="text-zinc-800 underline-offset-4 hover:underline dark:text-white">
                    #{website.subscription_code}
                  </Link>
                ) : (
                  '-'
                )}
              </Row>
            </dl>
          </Card>
          <Card title={t('Aan de slag')}>
            <div className="space-y-3 text-sm text-zinc-500 dark:text-white/70">
              <p>{t('Via {panel} beheer je je bestanden, e-mailadressen, databases en SSL-certificaten.', { panel })}</p>
              <p>{t('Laat de DNS van je domein naar deze server wijzen. De gegevens daarvoor vind je in {panel}.', { panel })}</p>
            </div>
          </Card>
        </div>
      )}
    </>
  )
}
