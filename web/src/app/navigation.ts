import {
  AdjustmentsHorizontalIcon,
  ArchiveBoxIcon,
  BookOpenIcon,
  CameraIcon,
  CircleStackIcon,
  CloudIcon,
  Cog6ToothIcon,
  CreditCardIcon,
  CubeIcon,
  DocumentDuplicateIcon,
  DocumentTextIcon,
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
  ShieldCheckIcon,
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
      { label: 'Helpcentrum', to: links.helpCenter, icon: BookOpenIcon, external: true },
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
    { label: 'Alle tickets', to: '/admin/tickets', icon: InboxStackIcon },
    { label: 'Alle facturen', to: '/admin/invoices', icon: DocumentDuplicateIcon },
    { label: 'Btw-overzicht', to: '/admin/vat', icon: ReceiptPercentIcon },
    { label: 'Diensten', to: '/admin/services', icon: ServerStackIcon },
    { label: 'Producten', to: '/admin/products', icon: CubeIcon },
    { label: 'Domeinextensies', to: '/admin/domain-extensions', icon: GlobeAltIcon },
    { label: 'Koppelingen', to: '/admin/integrations', icon: LinkIcon },
    { label: 'Instellingen', to: '/admin/settings', icon: AdjustmentsHorizontalIcon },
    { label: 'Systeem', to: '/admin/system', icon: Cog6ToothIcon },
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

export function navigationFor(isAdmin: boolean): NavEntry[] {
  return isAdmin ? [...navigation, adminNavigation] : navigation
}

export function flatNavigationFor(isAdmin: boolean): NavLink[] {
  return navigationFor(isAdmin).flatMap((entry) => (isGroup(entry) ? entry.children : [entry]))
}
