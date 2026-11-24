import { Dialog, DialogBackdrop, DialogPanel } from '@headlessui/react'
import { Bars2Icon, BellIcon, MagnifyingGlassIcon, XMarkIcon } from '@heroicons/react/24/outline'
import { useEffect, useState } from 'react'
import { Link, Navigate, Outlet, useLocation } from 'react-router'

import { useMe, type Profile } from '../../lib/auth'
import { cn } from '../../lib/cn'
import { Logo } from '../ui/Logo'
import { useNotifications } from '../../lib/notifications'
import { NavList } from './NavList'
import { itemBase, itemIdle } from './navStyles'
import { NotificationsPanel } from './NotificationsPanel'
import { SearchPalette } from './SearchPalette'
import { DemoBanner } from './DemoBanner'
import { UserMenu } from './UserMenu'

type SidebarProps = {
  profile: Profile
  onSearch: () => void
  onNotifications: () => void
  onNavigate?: () => void
}

function Sidebar({ profile, onSearch, onNotifications, onNavigate }: SidebarProps) {
  const { data: notifications } = useNotifications()

  return (
    <div className="flex h-full flex-col px-4 pt-4 pb-4">
      <Link to="/" onClick={onNavigate} className="flex h-9 items-center px-2">
        <Logo className="h-[22px] text-zinc-900 dark:text-white" />
      </Link>
      <button
        type="button"
        onClick={onSearch}
        className="relative mt-3 flex h-8 w-full items-center rounded-md border border-zinc-200 bg-white pr-4 pl-10 text-sm text-zinc-400 shadow-xs transition-colors hover:text-zinc-500 dark:border-transparent dark:bg-white/10 dark:text-white/40 dark:shadow-none dark:hover:text-white/60"
      >
        <MagnifyingGlassIcon className="absolute left-3 size-4" aria-hidden />
        <span className="flex-1 text-left">Zoeken...</span>
        <kbd className="font-sans text-xs text-zinc-400/75">Ctrl+K</kbd>
      </button>
      <div className="mt-4 min-h-0 flex-1 overflow-y-auto">
        <NavList isAdmin={profile.role === 'admin'} onNavigate={onNavigate} />
      </div>
      <div className="mt-4 space-y-4.5">
        <button type="button" onClick={onNotifications} className={cn(itemBase, itemIdle)}>
          <BellIcon className="size-4 shrink-0" aria-hidden />
          <span className="flex-1 text-left">Meldingen</span>
          <span className="rounded-sm bg-zinc-400/15 px-1 py-0.5 text-xs font-medium text-zinc-700 dark:bg-zinc-400/40 dark:text-zinc-200">
            {notifications?.unread_count ?? 0}
          </span>
        </button>
        <UserMenu profile={profile} />
      </div>
    </div>
  )
}

export function AppShell() {
  const { data: profile, isPending } = useMe()
  const location = useLocation()
  const [searchOpen, setSearchOpen] = useState(false)
  const [notificationsOpen, setNotificationsOpen] = useState(false)
  const [mobileNavOpen, setMobileNavOpen] = useState(false)

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setSearchOpen(true)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  if (isPending) {
    return null
  }
  if (!profile) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />
  }

  const sidebarProps = {
    profile,
    onSearch: () => {
      setMobileNavOpen(false)
      setSearchOpen(true)
    },
    onNotifications: () => {
      setMobileNavOpen(false)
      setNotificationsOpen(true)
    },
  }

  return (
    <div className="min-h-full">
      <aside className="fixed inset-y-0 left-0 hidden w-64 lg:block">
        <Sidebar {...sidebarProps} />
      </aside>

      <Dialog open={mobileNavOpen} onClose={setMobileNavOpen} className="relative z-50 lg:hidden">
        <DialogBackdrop transition className="fixed inset-0 bg-zinc-900/30 transition-opacity data-closed:opacity-0" />
        <DialogPanel transition className="fixed inset-y-0 left-0 w-72 border-r border-zinc-200 bg-zinc-50 transition duration-200 data-closed:-translate-x-full dark:border-white/10 dark:bg-zinc-900">
          <button type="button" onClick={() => setMobileNavOpen(false)} className="absolute top-5 right-4 rounded-md p-1 text-zinc-400 hover:text-zinc-800 dark:hover:text-white" aria-label="Menu sluiten">
            <XMarkIcon className="size-5" />
          </button>
          <Sidebar {...sidebarProps} onNavigate={() => setMobileNavOpen(false)} />
        </DialogPanel>
      </Dialog>

      <header className="flex h-14 items-center justify-between px-6 lg:hidden">
        <button type="button" onClick={() => setMobileNavOpen(true)} className="-ml-2 rounded-md p-2 text-zinc-500 hover:text-zinc-800 dark:text-white/80 dark:hover:text-white" aria-label="Menu openen">
          <Bars2Icon className="size-5" />
        </button>
        <UserMenu profile={profile} compact anchor="bottom end" />
      </header>

      <main className="lg:py-2 lg:pr-2 lg:pl-64">
        <div className="min-h-[calc(100vh-3.5rem)] bg-white p-6 sm:p-8 lg:min-h-[calc(100vh-1rem)] lg:rounded-xl lg:border lg:border-zinc-200 dark:bg-zinc-800 lg:dark:border-zinc-700">
          <DemoBanner profile={profile} />
          <Outlet />
        </div>
      </main>

      <SearchPalette open={searchOpen} onClose={() => setSearchOpen(false)} isAdmin={profile.role === 'admin'} />
      <NotificationsPanel open={notificationsOpen} onClose={() => setNotificationsOpen(false)} />
    </div>
  )
}
