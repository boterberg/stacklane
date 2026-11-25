import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { useSearchParams } from 'react-router'

import { ButtonLink } from '../../components/ui/Button'
import { api, ApiError } from '../../lib/api'
import { useMe } from '../../lib/auth'
import { AuthHeading } from './AuthLayout'
import { t } from '../../lib/i18n'

export function VerifyEmailPage() {
  const [params] = useSearchParams()
  const token = params.get('token') ?? ''
  const queryClient = useQueryClient()
  const { data: profile, isPending: checking } = useMe()
  const started = useRef(false)
  const verify = useMutation({
    mutationFn: () => api<void>('/auth/verify-email', { method: 'POST', body: { token } }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['me'] }),
  })

  useEffect(() => {
    if (!token || !profile || started.current) return
    started.current = true
    verify.mutate()
  }, [token, profile, verify])

  const error = verify.error instanceof ApiError ? (verify.error.fieldErrors.token ?? verify.error.message) : null
  const loggedOut = !checking && !profile && !!token

  return (
    <>
      <AuthHeading>{verify.isSuccess ? t('E-mailadres bevestigd') : t('E-mailadres bevestigen')}</AuthHeading>
      <p className="mb-6 text-center text-sm text-zinc-500 dark:text-white/70">
        {!token
          ? t('Deze link is niet compleet.')
          : loggedOut
            ? t('Log eerst in met het account waar dit e-mailadres bij hoort. Daarna bevestigen we het meteen.')
            : verify.isSuccess
              ? t('Bedankt. Je e-mailadres is bevestigd.')
              : error ?? t('Even geduld…')}
      </p>
      {loggedOut && (
        <ButtonLink to="/login" state={{ from: `/verify-email?token=${encodeURIComponent(token)}` }} variant="primary" className="w-full">
          {t('Naar inloggen')}
        </ButtonLink>
      )}
      {!loggedOut && (verify.isSuccess || verify.isError || !token) && (
        <ButtonLink to={profile ? '/' : '/login'} variant="primary" className="w-full">
          {profile ? t('Naar het dashboard') : t('Naar inloggen')}
        </ButtonLink>
      )}
    </>
  )
}
