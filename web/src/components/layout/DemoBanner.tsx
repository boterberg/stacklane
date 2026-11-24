import { ArrowPathIcon } from '@heroicons/react/24/outline'
import { useNavigate } from 'react-router'

import type { Profile } from '../../lib/auth'
import { useDemoLogin, useMeta } from '../../lib/meta'

const timeFormatter = new Intl.DateTimeFormat('nl-NL', { hour: '2-digit', minute: '2-digit' })

export function DemoBanner({ profile }: { profile: Profile }) {
  const { data: meta } = useMeta()
  const demoLogin = useDemoLogin()
  const navigate = useNavigate()

  if (!meta?.demo) return null

  const other = profile.role === 'admin' ? 'customer' : 'admin'
  const resetAt = meta.next_reset_at ? timeFormatter.format(new Date(meta.next_reset_at)) : null

  return (
    <div className="-mx-6 -mt-6 mb-6 flex flex-wrap items-center justify-between gap-3 border-b border-zinc-200 bg-zinc-50 px-6 py-2.5 text-sm sm:-mx-8 sm:-mt-8 sm:mb-8 sm:px-8 lg:rounded-t-xl dark:border-white/10 dark:bg-white/5">
      <p className="text-zinc-500 dark:text-white/70">
        <span className="font-medium text-zinc-800 dark:text-white">Demo</span> · ingelogd als {profile.role === 'admin' ? 'beheerder' : 'klant'}
        {resetAt && <> · alles wordt om {resetAt} teruggezet</>}
      </p>
      <button
        type="button"
        disabled={demoLogin.isPending}
        onClick={() => demoLogin.mutate(other, { onSuccess: () => navigate(other === 'admin' ? '/admin/tickets' : '/') })}
        className="flex items-center gap-2 font-medium text-zinc-800 hover:underline dark:text-white"
      >
        <ArrowPathIcon className="size-4" aria-hidden />
        Wissel naar {other === 'admin' ? 'beheerder' : 'klant'}
      </button>
    </div>
  )
}
