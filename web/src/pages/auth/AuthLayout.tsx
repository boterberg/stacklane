import type { ReactNode } from 'react'
import { Link, Navigate, Outlet } from 'react-router'

import { BrandLogo } from '../../components/ui/BrandLogo'
import { LanguageMenu, ThemeSwitch } from '../../components/ui/Preferences'
import { useMe } from '../../lib/auth'

export function AuthLayout({ guestsOnly = true }: { guestsOnly?: boolean }) {
  const { data: profile, isPending } = useMe()
  if (isPending) return null
  if (profile && guestsOnly) return <Navigate to="/" replace />

  return (
    <div className="relative flex min-h-full flex-col items-center justify-center px-4 py-16 sm:px-6">
      <div className="absolute top-4 right-4 sm:top-6 sm:right-6">
        <LanguageMenu />
      </div>
      <Link to="/login" className="mb-8">
        <BrandLogo className="h-7 max-w-56" />
      </Link>
      <div className="w-full max-w-md rounded-xl border border-zinc-200 bg-white p-6 shadow-xs sm:p-8 dark:border-white/10 dark:bg-zinc-800 dark:shadow-none">
        <Outlet />
      </div>
      <div className="mt-6 w-28">
        <ThemeSwitch />
      </div>
    </div>
  )
}

export function AuthHeading({ children }: { children: ReactNode }) {
  return <h1 className="mb-8 text-center text-xl font-medium text-zinc-800 dark:text-white">{children}</h1>
}

export function AuthFooter({ children }: { children: ReactNode }) {
  return <p className="mt-6 text-center text-sm text-zinc-500 dark:text-white/70">{children}</p>
}

export function TextLink({ to, state, children }: { to: string; state?: unknown; children: ReactNode }) {
  return (
    <Link to={to} state={state} className="font-medium text-zinc-800 underline decoration-zinc-800/20 underline-offset-4 hover:decoration-current dark:text-white dark:decoration-white/20">
      {children}
    </Link>
  )
}
