import { createBrowserRouter } from 'react-router'

import { AppShell } from '../components/layout/AppShell'
import { AuthLayout } from '../pages/auth/AuthLayout'
import { ForgotPasswordPage } from '../pages/auth/ForgotPasswordPage'
import { LoginPage } from '../pages/auth/LoginPage'
import { RegisterPage } from '../pages/auth/RegisterPage'
import { ResetPasswordPage } from '../pages/auth/ResetPasswordPage'
import { InvoiceDetailPage } from '../pages/billing/InvoiceDetailPage'
import { InvoicesPage } from '../pages/billing/InvoicesPage'
import { SubscriptionDetailPage } from '../pages/billing/SubscriptionDetailPage'
import { SubscriptionsPage } from '../pages/billing/SubscriptionsPage'
import { DashboardPage } from '../pages/DashboardPage'
import { NotFoundPage, SectionPage } from '../pages/SectionPage'
import { TicketDetailPage } from '../pages/support/TicketDetailPage'
import { TicketsPage } from '../pages/support/TicketsPage'
import { GameServerPage } from '../pages/gameservers/GameServerPage'
import { GameServersPage } from '../pages/gameservers/GameServersPage'
import { SecurityPage } from '../pages/account/SecurityPage'
import { IntegrationsPage } from '../pages/admin/IntegrationsPage'
import { AdminInvoicePage } from '../pages/admin/InvoicePage'
import { AdminInvoicesPage } from '../pages/admin/InvoicesPage'
import { SettingsPage } from '../pages/admin/SettingsPage'
import { ProductsPage } from '../pages/admin/ProductsPage'
import { ServicesPage } from '../pages/admin/ServicesPage'
import { SystemPage } from '../pages/admin/SystemPage'

const sections: Array<[string, string]> = [
  ['/gameservers/backups', 'Back-ups'],
  ['/vps/servers', 'Servers'],
  ['/domains', 'Domeinen'],
  ['/domains/whois-profiles', 'WHOIS-profielen'],
  ['/webhosting', 'Webhosting'],
  ['/account/profile', 'Account'],
]

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
      { path: '/admin/services', element: <ServicesPage /> },
      { path: '/admin/products', element: <ProductsPage /> },
      { path: '/admin/integrations', element: <IntegrationsPage /> },
      { path: '/account/security', element: <SecurityPage /> },
      { path: '/account/security/sessions', element: <SecurityPage /> },
      { path: '/admin/system', element: <SystemPage /> },
      { path: '/admin/settings', element: <SettingsPage /> },
      { path: '/admin/invoices', element: <AdminInvoicesPage /> },
      { path: '/admin/invoices/:id', element: <AdminInvoicePage /> },
      { path: '/admin/tickets', element: <TicketsPage scope="staff" /> },
      { path: '/admin/tickets/:number', element: <TicketDetailPage scope="staff" /> },
      ...sections.map(([path, title]) => ({ path, element: <SectionPage title={title} /> })),
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
