import { createBrowserRouter, Navigate } from 'react-router'

import { AppShell } from '../components/layout/AppShell'
import { AuthLayout } from '../pages/auth/AuthLayout'
import { ForgotPasswordPage } from '../pages/auth/ForgotPasswordPage'
import { LoginPage } from '../pages/auth/LoginPage'
import { RegisterPage } from '../pages/auth/RegisterPage'
import { ResetPasswordPage } from '../pages/auth/ResetPasswordPage'
import { VerifyEmailPage } from '../pages/auth/VerifyEmailPage'
import { InvoiceDetailPage } from '../pages/billing/InvoiceDetailPage'
import { InvoicesPage } from '../pages/billing/InvoicesPage'
import { SubscriptionDetailPage } from '../pages/billing/SubscriptionDetailPage'
import { SubscriptionsPage } from '../pages/billing/SubscriptionsPage'
import { DashboardPage } from '../pages/DashboardPage'
import { NotFoundPage, SectionPage } from '../pages/SectionPage'
import { TicketDetailPage } from '../pages/support/TicketDetailPage'
import { TicketsPage } from '../pages/support/TicketsPage'
import { GameServerPage } from '../pages/gameservers/GameServerPage'
import { DomainExtensionsPage } from '../pages/admin/DomainExtensionsPage'
import { DomainPage } from '../pages/domains/DomainPage'
import { DomainsPage } from '../pages/domains/DomainsPage'
import { RegisterDomainPage } from '../pages/domains/RegisterDomainPage'
import { WhoisProfilesPage } from '../pages/domains/WhoisProfilesPage'
import { OrderServerPage } from '../pages/vps/OrderServerPage'
import { VpsServerPage } from '../pages/vps/ServerPage'
import { VpsServersPage } from '../pages/vps/ServersPage'
import { FirewallPage } from '../pages/vps/FirewallPage'
import { FirewallsPage } from '../pages/vps/FirewallsPage'
import { FloatingIpsPage } from '../pages/vps/FloatingIpsPage'
import { NetworkPage } from '../pages/vps/NetworkPage'
import { NetworksPage } from '../pages/vps/NetworksPage'
import { IsosPage } from '../pages/vps/IsosPage'
import { UsersPage } from '../pages/vps/UsersPage'
import { SnapshotsPage } from '../pages/vps/SnapshotsPage'
import { VolumesPage } from '../pages/vps/VolumesPage'
import { OrderWebhostingPage } from '../pages/webhosting/OrderWebhostingPage'
import { WebhostingPage } from '../pages/webhosting/WebhostingPage'
import { WebsitePage } from '../pages/webhosting/WebsitePage'
import { SshKeysPage } from '../pages/vps/SshKeysPage'
import { GameServersPage } from '../pages/gameservers/GameServersPage'
import { AccountPage } from '../pages/account/AccountPage'
import { SecurityPage } from '../pages/account/SecurityPage'
import { SharedAccessPage } from '../pages/account/SharedAccessPage'
import { IntegrationsPage } from '../pages/admin/IntegrationsPage'
import { AdminInvoicePage } from '../pages/admin/InvoicePage'
import { AdminInvoicesPage } from '../pages/admin/InvoicesPage'
import { MailLogPage } from '../pages/admin/MailLogPage'
import { VatReportPage } from '../pages/admin/VatReportPage'
import { SettingsPage } from '../pages/admin/SettingsPage'
import { ProductsPage } from '../pages/admin/ProductsPage'
import { ServicesPage } from '../pages/admin/ServicesPage'
import { SystemPage } from '../pages/admin/SystemPage'

const sections: Array<[string, string]> = [['/gameservers/backups', 'Back-ups']]

export const router = createBrowserRouter([
  {
    element: <AuthLayout />,
    children: [
      { path: '/login', element: <LoginPage /> },
      { path: '/register', element: <RegisterPage /> },
      { path: '/forgot-password', element: <ForgotPasswordPage /> },
      { path: '/reset-password', element: <ResetPasswordPage /> },
    ],
  },
  {
    element: <AuthLayout guestsOnly={false} />,
    children: [{ path: '/verify-email', element: <VerifyEmailPage /> }],
  },
  {
    element: <AppShell />,
    children: [
      { path: '/', element: <DashboardPage /> },
      { path: '/support/tickets', element: <TicketsPage /> },
      { path: '/billing/subscriptions', element: <SubscriptionsPage /> },
      { path: '/billing/subscriptions/:code', element: <SubscriptionDetailPage /> },
      { path: '/billing/invoices', element: <InvoicesPage /> },
      { path: '/billing/invoices/:id', element: <InvoiceDetailPage /> },
      { path: '/support/tickets/:number', element: <TicketDetailPage /> },
      { path: '/gameservers/servers', element: <GameServersPage /> },
      { path: '/gameservers/servers/:id', element: <GameServerPage /> },
      { path: '/admin/domain-extensions', element: <DomainExtensionsPage /> },
      { path: '/domains', element: <DomainsPage /> },
      { path: '/domains/register', element: <RegisterDomainPage /> },
      { path: '/domains/:id', element: <DomainPage /> },
      { path: '/domains/whois-profiles', element: <WhoisProfilesPage /> },
      { path: '/vps', element: <Navigate to="/vps/servers" replace /> },
      { path: '/webhosting', element: <WebhostingPage /> },
      { path: '/webhosting/order', element: <OrderWebhostingPage /> },
      { path: '/webhosting/:id', element: <WebsitePage /> },
      { path: '/vps/servers', element: <VpsServersPage /> },
      { path: '/vps/servers/order', element: <OrderServerPage /> },
      { path: '/vps/servers/:id', element: <VpsServerPage /> },
      { path: '/vps/volumes', element: <VolumesPage /> },
      { path: '/vps/snapshots', element: <SnapshotsPage /> },
      { path: '/vps/isos', element: <IsosPage /> },
      { path: '/vps/floating-ips', element: <FloatingIpsPage /> },
      { path: '/vps/networks', element: <NetworksPage /> },
      { path: '/vps/networks/:id', element: <NetworkPage /> },
      { path: '/vps/firewalls', element: <FirewallsPage /> },
      { path: '/vps/firewalls/:id', element: <FirewallPage /> },
      { path: '/vps/ssh-keys', element: <SshKeysPage /> },
      { path: '/vps/users', element: <UsersPage /> },
      { path: '/account/shared-access', element: <SharedAccessPage /> },
      { path: '/admin/services', element: <ServicesPage /> },
      { path: '/admin/products', element: <ProductsPage /> },
      { path: '/admin/integrations', element: <IntegrationsPage /> },
      { path: '/account/profile', element: <AccountPage /> },
      { path: '/account/profile/settings', element: <AccountPage /> },
      { path: '/account/payment-methods', element: <AccountPage /> },
      { path: '/account/profile/delete', element: <AccountPage /> },
      { path: '/account/security', element: <SecurityPage /> },
      { path: '/account/security/two-factor', element: <SecurityPage /> },
      { path: '/account/security/sessions', element: <SecurityPage /> },
      { path: '/account/security/api-tokens', element: <SecurityPage /> },
      { path: '/admin/system', element: <SystemPage /> },
      { path: '/admin/settings', element: <SettingsPage /> },
      { path: '/admin/vat', element: <VatReportPage /> },
      { path: '/admin/mail-log', element: <MailLogPage /> },
      { path: '/admin/invoices', element: <AdminInvoicesPage /> },
      { path: '/admin/invoices/:id', element: <AdminInvoicePage /> },
      { path: '/admin/tickets', element: <TicketsPage scope="staff" /> },
      { path: '/admin/tickets/:number', element: <TicketDetailPage scope="staff" /> },
      ...sections.map(([path, title]) => ({ path, element: <SectionPage title={title} /> })),
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
