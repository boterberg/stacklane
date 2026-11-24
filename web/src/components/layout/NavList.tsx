import { ArrowTopRightOnSquareIcon, ChevronDownIcon, ChevronLeftIcon, ChevronRightIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { Link, NavLink as RouterNavLink, useLocation } from 'react-router'

import { isGroup, navigationFor, sectionFor, type NavGroup, type NavLink, type NavSection } from '../../app/navigation'
import { cn } from '../../lib/cn'
import { itemActive, itemBase, itemIdle } from './navStyles'
import { t } from '../../lib/i18n'
import { useCan, useSwitchEnvironment, useVpsAccess } from '../../lib/vpsAccess'

function isActivePath(pathname: string, to: string) {
  return to === '/' ? pathname === '/' : pathname === to || pathname.startsWith(`${to}/`)
}

function LeafItem({ item, onNavigate }: { item: NavLink; onNavigate?: () => void }) {
  const Icon = item.icon
  if (item.external) {
    return (
      <a href={item.to} target="_blank" rel="noreferrer" className={cn(itemBase, itemIdle)}>
        <Icon className="size-4 shrink-0" aria-hidden />
        <span className="flex-1 truncate">{t(item.label)}</span>
        <ArrowTopRightOnSquareIcon className="size-4 shrink-0" aria-hidden />
      </a>
    )
  }
  return (
    <RouterNavLink to={item.to} end={item.to === '/' || item.exact} onClick={onNavigate} className={({ isActive }) => cn(itemBase, isActive ? itemActive : itemIdle)}>
      <Icon className="size-4 shrink-0" aria-hidden />
      <span className="flex-1 truncate">{t(item.label)}</span>
      {item.opensSection && <ChevronRightIcon className="size-3 shrink-0" strokeWidth={2} aria-hidden />}
    </RouterNavLink>
  )
}

function GroupItem({ group, onNavigate }: { group: NavGroup; onNavigate?: () => void }) {
  const { pathname } = useLocation()
  const containsActive = group.children.some((child) => !child.external && isActivePath(pathname, child.to))
  const [open, setOpen] = useState(containsActive)
  const Icon = group.icon
  const Chevron = open ? ChevronDownIcon : ChevronRightIcon

  return (
    <div>
      <button type="button" onClick={() => setOpen(!open)} aria-expanded={open} className={cn(itemBase, itemIdle)}>
        <Icon className="size-4 shrink-0" aria-hidden />
        <span className="flex-1 truncate text-left">{t(group.label)}</span>
        <Chevron className="size-3 shrink-0" strokeWidth={2} aria-hidden />
      </button>
      {open && (
        <div className="mt-0.5 ml-4 space-y-0.5 border-l border-zinc-200 pl-3 dark:border-white/20">
          {group.children.map((child) => (
            <LeafItem key={child.label} item={child} onNavigate={onNavigate} />
          ))}
        </div>
      )}
    </div>
  )
}

function SectionNav({ section, onNavigate }: { section: NavSection; onNavigate?: () => void }) {
  const can = useCan()
  const switchTo = useSwitchEnvironment()
  const { data: access } = useVpsAccess()
  const groups = section.groups.map((group) => ({ ...group, children: group.children.filter((child) => !child.permission || can(child.permission)) })).filter((group) => group.children.length > 0)
  return (
    <nav>
      <div className="relative mb-2 flex h-8 items-center justify-center">
        <Link
          to={section.back}
          onClick={onNavigate}
          aria-label={t('Terug')}
          className="absolute left-1 rounded-md p-1.5 text-zinc-500 hover:bg-zinc-800/5 hover:text-zinc-800 dark:text-white/70 dark:hover:bg-white/10 dark:hover:text-white"
        >
          <ChevronLeftIcon className="size-3.5" strokeWidth={2} aria-hidden />
        </Link>
        <span className="text-sm font-medium text-zinc-800 dark:text-white">{t(section.title)}</span>
      </div>
      {access && !access.is_owner && (
        <div className="mx-1 mb-2 rounded-lg bg-zinc-800/5 px-3 py-2 text-sm dark:bg-white/10">
          <p className="text-zinc-500 dark:text-white/60">{t('Omgeving van')}</p>
          <p className="truncate font-medium text-zinc-800 dark:text-white">{access.owner_name}</p>
          <button type="button" className="mt-1 text-zinc-500 underline hover:text-zinc-800 dark:text-white/60 dark:hover:text-white" onClick={() => switchTo(null)}>
            {t('Naar je eigen omgeving')}
          </button>
        </div>
      )}
      {groups.map((group) => (
        <div key={group.label} className="mb-1">
          <p className="px-3 pt-2 pb-1.5 text-sm text-zinc-400 dark:text-white/50">{t(group.label)}</p>
          <div className="space-y-0.5">
            {group.children.map((child) => (
              <LeafItem key={child.label} item={child} onNavigate={onNavigate} />
            ))}
          </div>
        </div>
      ))}
    </nav>
  )
}

export function NavList({ isAdmin, onNavigate }: { isAdmin: boolean; onNavigate?: () => void }) {
  const { pathname } = useLocation()
  const section = sectionFor(pathname)
  if (section) return <SectionNav section={section} onNavigate={onNavigate} />
  return (
    <nav className="space-y-0.5">
      {navigationFor(isAdmin).map((entry) =>
        isGroup(entry) ? <GroupItem key={entry.label} group={entry} onNavigate={onNavigate} /> : <LeafItem key={entry.label} item={entry} onNavigate={onNavigate} />,
      )}
    </nav>
  )
}
