import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { ArrowRightStartOnRectangleIcon, ChevronDownIcon, ShieldCheckIcon, UserIcon } from '@heroicons/react/24/outline'
import { Link, useNavigate } from 'react-router'

import { displayName, initials, useLogout, type Profile } from '../../lib/auth'
import { cn } from '../../lib/cn'

const menuItem = 'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium text-zinc-800 data-focus:bg-zinc-50 dark:text-white dark:data-focus:bg-zinc-600'

export function Avatar({ profile }: { profile: Profile }) {
  return (
    <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-zinc-200 text-sm font-medium text-zinc-800 dark:bg-zinc-600 dark:text-white">
      {initials(profile)}
    </span>
  )
}

export function UserMenu({ profile, compact = false, anchor = 'top start' }: { profile: Profile; compact?: boolean; anchor?: 'top start' | 'bottom end' }) {
  const logout = useLogout()
  const navigate = useNavigate()

  return (
    <Menu>
      <MenuButton
        className={cn(
          'flex items-center gap-2 rounded-lg p-1 text-left transition-colors hover:bg-zinc-800/5 dark:hover:bg-white/10',
          compact ? '' : 'w-full',
        )}
      >
        <Avatar profile={profile} />
        <span className={cn('truncate text-sm font-medium text-zinc-500 dark:text-white/80', compact ? 'max-sm:hidden' : 'flex-1')}>{displayName(profile)}</span>
        <ChevronDownIcon className="mr-2 size-4 shrink-0 text-zinc-400 dark:text-white/80" aria-hidden />
      </MenuButton>
      <MenuItems
        anchor={{ to: anchor, gap: 8 }}
        className="z-50 w-(--button-width) min-w-48 rounded-lg border border-zinc-200 bg-white p-1 shadow-xs focus:outline-none dark:border-zinc-600 dark:bg-zinc-700"
      >
        <MenuItem>
          <Link to="/account/profile" className={menuItem}>
            <UserIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
            Account
          </Link>
        </MenuItem>
        <MenuItem>
          <Link to="/account/security" className={menuItem}>
            <ShieldCheckIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
            Beveiliging
          </Link>
        </MenuItem>
        <div className="-mx-1 my-1 h-px bg-zinc-200 dark:bg-zinc-600" />
        <MenuItem>
          <button type="button" className={menuItem} onClick={() => logout.mutate(undefined, { onSettled: () => navigate('/login') })}>
            <ArrowRightStartOnRectangleIcon className="size-4 text-zinc-400 dark:text-white/60" aria-hidden />
            Uitloggen
          </button>
        </MenuItem>
      </MenuItems>
    </Menu>
  )
}
