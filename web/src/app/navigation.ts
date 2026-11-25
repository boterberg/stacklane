import {
  AdjustmentsHorizontalIcon,
  ArchiveBoxIcon,
  BookOpenIcon,
  CameraIcon,
  ChartBarIcon,
  CircleStackIcon,
  CloudIcon,
  Cog6ToothIcon,
  CreditCardIcon,
  CubeIcon,
  DocumentDuplicateIcon,
  DocumentTextIcon,
  EnvelopeIcon,
  GlobeAltIcon,
  GlobeEuropeAfricaIcon,
  HomeIcon,
  IdentificationIcon,
  InboxStackIcon,
  KeyIcon,
  LifebuoyIcon,
  LinkIcon,
  MapPinIcon,
  PlayCircleIcon,
  QuestionMarkCircleIcon,
  ReceiptPercentIcon,
  ServerStackIcon,
  ShareIcon,
  ShieldCheckIcon,
  UserGroupIcon,
  UsersIcon,
  WindowIcon,
  WrenchScrewdriverIcon,
} from '@heroicons/react/24/outline'
import type { ComponentType, SVGProps } from 'react'

import { links } from './links'

export type NavIcon = ComponentType<SVGProps<SVGSVGElement>>

export type NavLink = {
  label: string
  to: string
  icon: NavIcon
  external?: boolean
  opensSection?: boolean
  exact?: boolean
  permission?: string
  grant?: string
}

export type NavGroup = {
  label: string
  icon: NavIcon
  children: NavLink[]
}

export type NavEntry = NavLink | NavGroup

export function isGroup(entry: NavEntry): entry is NavGroup {
  return 'children' in entry
}

export const navigation: NavEntry[] = [
  { label: 'Dashboard', to: '/', icon: HomeIcon },
  {
    label: 'Gameservers',
    icon: CircleStackIcon,
    children: [
      { label: 'Servers', to: '/gameservers/servers', icon: ServerStackIcon },
      { label: 'Back-ups', to: '/gameservers/backups', icon: ArchiveBoxIcon },
    ],
  },
  { label: 'VPS', to: '/vps/servers', icon: CloudIcon, opensSection: true },
  {
    label: 'Domeinnamen',
    icon: GlobeAltIcon,
    children: [
      { label: 'Domeinen', to: '/domains', icon: GlobeEuropeAfricaIcon, exact: true },
      { label: 'WHOIS-profielen', to: '/domains/whois-profiles', icon: IdentificationIcon },
    ],
  },
  { label: 'Webhosting', to: '/webhosting', icon: WindowIcon },
  {
    label: 'Ondersteuning',
    icon: LifebuoyIcon,
    children: [
      { label: 'Tickets', to: '/support/tickets', icon: QuestionMarkCircleIcon },
      { label: 'Helpcentrum', to: links.helpCenter, icon: BookOpenIcon, external: links.helpCenter.startsWith('http') },
    ],
  },
  {
    label: 'Facturering',
    icon: CreditCardIcon,
    children: [
      { label: 'Abonnementen', to: '/billing/subscriptions', icon: CreditCardIcon },
      { label: 'Facturen', to: '/billing/invoices', icon: DocumentTextIcon },
    ],
  },
]

export const adminNavigation: NavGroup = {
  label: 'Beheer',
  icon: WrenchScrewdriverIcon,
  children: [
    { label: 'Statistieken', to: '/admin/stats', icon: ChartBarIcon, grant: 'billing' },
    { label: 'Klanten', to: '/admin/customers', icon: UsersIcon, grant: 'customers' },
    { label: 'Alle tickets', to: '/admin/tickets', icon: InboxStackIcon, grant: 'support' },
    { label: 'Alle facturen', to: '/admin/invoices', icon: DocumentDuplicateIcon, grant: 'billing' },
    { label: 'Btw-overzicht', to: '/admin/vat', icon: ReceiptPercentIcon, grant: 'billing' },
    { label: 'Diensten', to: '/admin/services', icon: ServerStackIcon, grant: 'services' },
    { label: 'Producten', to: '/admin/products', icon: CubeIcon, grant: 'services' },
    { label: 'Domeinextensies', to: '/admin/domain-extensions', icon: GlobeAltIcon, grant: 'services' },
    { label: 'Koppelingen', to: '/admin/integrations', icon: LinkIcon, grant: 'admin' },
    { label: 'Kennisbank', to: '/admin/kb', icon: BookOpenIcon, grant: 'support' },
    { label: 'Maillog', to: '/admin/mail-log', icon: EnvelopeIcon, grant: 'admin' },
    { label: 'Team', to: '/admin/team', icon: UserGroupIcon, grant: 'admin' },
    { label: 'Instellingen', to: '/admin/settings', icon: AdjustmentsHorizontalIcon, grant: 'admin' },
    { label: 'Systeem', to: '/admin/system', icon: Cog6ToothIcon, grant: 'admin' },
  ],
}

export type NavSection = {
  title: string
  back: string
  groups: Array<{ label: string; children: NavLink[] }>
}

export const vpsSection: NavSection = {
  title: 'VPS',
  back: '/',
  groups: [
    { label: 'Compute', children: [{ label: 'Servers', to: '/vps/servers', icon: ServerStackIcon, permission: 'servers.view' }] },
    {
      label: 'Opslag',
      children: [
        { label: 'Volumes', to: '/vps/volumes', icon: CircleStackIcon, permission: 'volumes.view' },
        { label: 'Snapshots', to: '/vps/snapshots', icon: CameraIcon, permission: 'snapshots.view' },
        { label: "ISO's", to: '/vps/isos', icon: PlayCircleIcon, permission: 'isos.view' },
      ],
    },
    {
      label: 'Netwerk',
      children: [
        { label: 'Floating IPs', to: '/vps/floating-ips', icon: MapPinIcon, permission: 'floating_ips.view' },
        { label: 'Netwerken', to: '/vps/networks', icon: ShareIcon, permission: 'networks.view' },
        { label: 'Firewalls', to: '/vps/firewalls', icon: ShieldCheckIcon, permission: 'firewalls.view' },
      ],
    },
    {
      label: 'Toegang',
      children: [
        { label: 'SSH-sleutels', to: '/vps/ssh-keys', icon: KeyIcon, permission: 'ssh_keys.view' },
        { label: 'Gebruikers', to: '/vps/users', icon: UsersIcon },
      ],
    },
  ],
}

export function sectionFor(pathname: string): NavSection | null {
  return pathname === '/vps' || pathname.startsWith('/vps/') ? vpsSection : null
}

export function navigationFor(grants: string[]): NavEntry[] {
  const children = adminNavigation.children.filter((child) => child.grant && grants.includes(child.grant))
  return children.length ? [...navigation, { ...adminNavigation, children }] : navigation
}

export function flatNavigationFor(grants: string[]): NavLink[] {
  return navigationFor(grants).flatMap((entry) => (isGroup(entry) ? entry.children : [entry]))
}
