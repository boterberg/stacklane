import { ArrowTopRightOnSquareIcon, ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'
import { NavLink as RouterNavLink, useLocation } from 'react-router'

import { isGroup, navigation, type NavGroup, type NavLink } from '../../app/navigation'
import { cn } from '../../lib/cn'
import { itemActive, itemBase, itemIdle } from './navStyles'

function isActivePath(pathname: string, to: string) {
  return to === '/' ? pathname === '/' : pathname === to || pathname.startsWith(`${to}/`)
}

function LeafItem({ item, onNavigate }: { item: NavLink; onNavigate?: () => void }) {
  const Icon = item.icon
  if (item.external) {
    return (
      <a href={item.to} target="_blank" rel="noreferrer" className={cn(itemBase, itemIdle)}>
        <Icon className="size-4 shrink-0" aria-hidden />
        <span className="flex-1 truncate">{item.label}</span>
        <ArrowTopRightOnSquareIcon className="size-4 shrink-0" aria-hidden />
      </a>
    )
  }
  return (
    <RouterNavLink to={item.to} end={item.to === '/'} onClick={onNavigate} className={({ isActive }) => cn(itemBase, isActive ? itemActive : itemIdle)}>
      <Icon className="size-4 shrink-0" aria-hidden />
      <span className="flex-1 truncate">{item.label}</span>
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
        <span className="flex-1 truncate text-left">{group.label}</span>
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

export function NavList({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav className="space-y-0.5">
      {navigation.map((entry) =>
        isGroup(entry) ? <GroupItem key={entry.label} group={entry} onNavigate={onNavigate} /> : <LeafItem key={entry.label} item={entry} onNavigate={onNavigate} />,
      )}
    </nav>
  )
}

