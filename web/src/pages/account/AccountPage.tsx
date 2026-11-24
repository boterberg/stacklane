import { useLocation, useNavigate } from 'react-router'

import { PageHeader } from '../../components/ui/Heading'
import { Tabs } from '../../components/ui/Tabs'
import { t } from '../../lib/i18n'
import { ProfileTab } from './ProfileTab'
import { SettingsTab } from './SettingsTab'

const tabs = [
  { id: '/account/profile', label: 'Profiel' },
  { id: '/account/profile/settings', label: 'Instellingen' },
]

export function AccountPage() {
  const { pathname } = useLocation()
  const navigate = useNavigate()
  const active = tabs.find((tab) => tab.id === pathname)?.id ?? tabs[0].id

  return (
    <>
      <PageHeader title={t('Account')} />
      <Tabs tabs={tabs} active={active} onChange={(id) => navigate(id)} />
      {active === '/account/profile' && <ProfileTab />}
      {active === '/account/profile/settings' && <SettingsTab />}
    </>
  )
}
