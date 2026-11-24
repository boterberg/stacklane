import { useLocation, useNavigate } from 'react-router'

import { PageHeader } from '../../components/ui/Heading'
import { Tabs } from '../../components/ui/Tabs'
import { t } from '../../lib/i18n'
import { ApiTokensTab } from './ApiTokensTab'
import { PasswordTab } from './PasswordTab'
import { SessionsTab } from './SessionsTab'

const tabs = [
  { id: '/account/security', label: 'Wachtwoord' },
  { id: '/account/security/sessions', label: 'Sessies' },
  { id: '/account/security/api-tokens', label: 'API-tokens' },
]

export function SecurityPage() {
  const { pathname } = useLocation()
  const navigate = useNavigate()
  const active = tabs.find((tab) => tab.id === pathname)?.id ?? tabs[0].id

  return (
    <>
      <PageHeader title={t('Beveiliging')} />
      <Tabs tabs={tabs} active={active} onChange={(id) => navigate(id)} />
      {active === '/account/security' && <PasswordTab />}
      {active === '/account/security/sessions' && <SessionsTab />}
      {active === '/account/security/api-tokens' && <ApiTokensTab />}
    </>
  )
}
