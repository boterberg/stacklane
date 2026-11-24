import { createBrowserRouter } from 'react-router'

import { AppShell } from '../components/layout/AppShell'
import { AuthLayout } from '../pages/auth/AuthLayout'
import { ForgotPasswordPage } from '../pages/auth/ForgotPasswordPage'
import { LoginPage } from '../pages/auth/LoginPage'
import { RegisterPage } from '../pages/auth/RegisterPage'
import { DashboardPage } from '../pages/DashboardPage'
import { NotFoundPage, SectionPage } from '../pages/SectionPage'
import { TicketDetailPage } from '../pages/support/TicketDetailPage'
import { TicketsPage } from '../pages/support/TicketsPage'

const sections: Array<[string, string]> = [
  ['/gameservers/servers', 'Servers'],
  ['/gameservers/backups', 'Back-ups'],
  ['/vps/servers', 'Servers'],
  ['/domains', 'Domeinen'],
  ['/domains/whois-profiles', 'WHOIS-profielen'],
  ['/webhosting', 'Webhosting'],
  ['/billing/subscriptions', 'Abonnementen'],
  ['/billing/invoices', 'Facturen'],
  ['/account/profile', 'Account'],
  ['/account/security', 'Beveiliging'],
]

export const router = createBrowserRouter([
  {
    element: <AuthLayout />,
    children: [
      { path: '/login', element: <LoginPage /> },
      { path: '/register', element: <RegisterPage /> },
      { path: '/forgot-password', element: <ForgotPasswordPage /> },
    ],
  },
  {
    element: <AppShell />,
    children: [
      { path: '/', element: <DashboardPage /> },
      { path: '/support/tickets', element: <TicketsPage /> },
      { path: '/support/tickets/:number', element: <TicketDetailPage /> },
      ...sections.map(([path, title]) => ({ path, element: <SectionPage title={title} /> })),
      { path: '*', element: <NotFoundPage /> },
    ],
  },
])
