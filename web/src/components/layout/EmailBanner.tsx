import { EnvelopeIcon } from '@heroicons/react/24/outline'

import { useResendVerification } from '../../lib/account'
import type { Profile } from '../../lib/auth'
import { useMeta } from '../../lib/meta'
import { t } from '../../lib/i18n'

export function EmailBanner({ profile }: { profile: Profile }) {
  const { data: meta } = useMeta()
  const resend = useResendVerification()

  if (profile.email_verified || meta?.demo) return null

  return (
    <div className="-mx-6 -mt-6 mb-6 flex flex-wrap items-center justify-between gap-3 border-b border-zinc-200 bg-zinc-50 px-6 py-2.5 text-sm sm:-mx-8 sm:-mt-8 sm:mb-8 sm:px-8 lg:rounded-t-xl dark:border-white/10 dark:bg-white/5">
      <p className="flex items-center gap-2 text-zinc-500 dark:text-white/70">
        <EnvelopeIcon className="size-4 shrink-0" aria-hidden />
        <span>{t('Bevestig je e-mailadres via de link die we naar {email} stuurden.', { email: profile.email })}</span>
      </p>
      <button type="button" disabled={resend.isPending || resend.isSuccess} onClick={() => resend.mutate()} className="font-medium text-zinc-800 hover:underline disabled:no-underline disabled:opacity-60 dark:text-white">
        {resend.isSuccess ? t('Opnieuw gestuurd') : t('Stuur opnieuw')}
      </button>
    </div>
  )
}
