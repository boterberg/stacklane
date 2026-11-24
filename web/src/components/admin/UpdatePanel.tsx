import { Dialog, DialogBackdrop, DialogPanel, DialogTitle } from '@headlessui/react'
import { ArrowPathIcon, CheckCircleIcon, ExclamationTriangleIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { ApiError } from '../../lib/api'
import { t } from '../../lib/i18n'
import { isRunning, justFinished, releaseNotesUrl, useStartUpdate, type UpdateOverview, type UpdateProgress } from '../../lib/system'
import { Button } from '../ui/Button'

const steps: Array<UpdateProgress['step']> = ['checking', 'backup', 'installing', 'restarting']

const stepLabels: Record<UpdateProgress['step'], string> = {
  queued: 'Wacht op de server',
  checking: 'Pakketbron controleren',
  backup: 'Back-up van de database maken',
  installing: 'Nieuwe versie installeren',
  restarting: 'Stacklane opnieuw starten',
  done: 'Klaar',
  failed: 'Mislukt',
}

function ConfirmDialog({ open, version, onClose, onConfirm, pending }: { open: boolean; version: string; onClose: () => void; onConfirm: () => void; pending: boolean }) {
  return (
    <Dialog open={open} onClose={onClose} className="relative z-50">
      <DialogBackdrop transition className="fixed inset-0 bg-zinc-900/20 transition-opacity data-closed:opacity-0 dark:bg-black/40" />
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <DialogPanel transition className="w-full max-w-md rounded-xl border border-zinc-200 bg-white p-6 shadow-xl transition data-closed:scale-95 data-closed:opacity-0 dark:border-white/10 dark:bg-zinc-800">
          <DialogTitle className="text-base font-medium text-zinc-800 dark:text-white">{t('Bijwerken naar {version}?', { version })}</DialogTitle>
          <p className="mt-2 text-sm text-zinc-500 dark:text-white/70">
            {t('Stacklane maakt eerst een back-up van de database en installeert daarna de nieuwe versie. Tijdens het herstarten is het panel een halve minuut niet bereikbaar.')}
          </p>
          <div className="mt-6 flex justify-end gap-2">
            <Button onClick={onClose}>{t('Annuleren')}</Button>
            <Button variant="primary" onClick={onConfirm} disabled={pending}>
              {t('Nu bijwerken')}
            </Button>
          </div>
        </DialogPanel>
      </div>
    </Dialog>
  )
}

function Progress({ overview }: { overview: UpdateOverview }) {
  const progress = overview.progress!
  const finished = progress.state === 'done' && overview.current === progress.target
  if (finished) {
    return (
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="flex items-center gap-2 text-sm font-medium text-zinc-800 dark:text-white">
          <CheckCircleIcon className="size-5 text-green-600 dark:text-green-400" aria-hidden />
          {t('Stacklane is bijgewerkt naar {version}.', { version: overview.current })}
        </p>
        <Button size="sm" icon={ArrowPathIcon} onClick={() => window.location.reload()}>
          {t('Pagina vernieuwen')}
        </Button>
      </div>
    )
  }
  if (progress.state === 'failed') {
    return (
      <div className="text-sm">
        <p className="flex items-center gap-2 font-medium text-red-600 dark:text-red-400">
          <ExclamationTriangleIcon className="size-5" aria-hidden />
          {t('Bijwerken is mislukt.')}
        </p>
        {progress.message && <p className="mt-2 font-mono text-xs text-zinc-500 dark:text-white/60">{progress.message}</p>}
        <p className="mt-2 text-zinc-500 dark:text-white/70">{t('Er is niets kapotgegaan aan je gegevens. Meer details vind je met journalctl -u stacklane-update.')}</p>
      </div>
    )
  }
  const currentIndex = steps.indexOf(progress.step)
  return (
    <ol className="space-y-2">
      {steps.map((step, index) => {
        const done = progress.state === 'done' || index < currentIndex
        const active = isRunning(progress) && index === currentIndex
        return (
          <li key={step} className="flex items-center gap-3 text-sm">
            <span
              className={
                done
                  ? 'size-2 rounded-full bg-green-500'
                  : active
                    ? 'size-2 animate-pulse rounded-full bg-amber-500'
                    : 'size-2 rounded-full bg-zinc-300 dark:bg-white/20'
              }
            />
            <span className={done || active ? 'font-medium text-zinc-800 dark:text-white' : 'text-zinc-500 dark:text-white/60'}>{t(stepLabels[step])}</span>
          </li>
        )
      })}
      {progress.step === 'queued' && <li className="text-sm text-zinc-500 dark:text-white/60">{t(stepLabels.queued)}</li>}
    </ol>
  )
}

export function UpdatePanel({ overview, demo }: { overview: UpdateOverview; demo: boolean }) {
  const [confirming, setConfirming] = useState(false)
  const start = useStartUpdate()
  const error = start.error instanceof ApiError ? (start.error.fieldErrors.update ?? start.error.message) : null
  const progress = overview.progress
  if (progress && (isRunning(progress) || justFinished(overview))) {
    return <Progress overview={overview} />
  }
  const failed = progress?.state === 'failed' && progress.target === overview.latest

  return (
    <div className="space-y-4">
      {failed && <Progress overview={overview} />}
      {overview.available && overview.latest && (
        <>
          <p className="text-sm text-zinc-500 dark:text-white/70">
            {t('Versie {latest} staat klaar. Je draait nu {current}.', { latest: overview.latest, current: overview.current })}{' '}
            <a href={releaseNotesUrl(overview.latest)} target="_blank" rel="noreferrer" className="font-medium text-zinc-800 underline underline-offset-4 dark:text-white">
              {t('Wat is er nieuw')}
            </a>
          </p>
          {overview.can_update && !demo ? (
            <Button variant="primary" size="sm" onClick={() => setConfirming(true)}>
              {t('Nu bijwerken')}
            </Button>
          ) : (
            <p className="text-sm text-zinc-500 dark:text-white/70">
              {demo ? t('In de demo kun je niet bijwerken.') : t('Bijwerken vanuit het panel is op deze server niet ingesteld. Werk bij met:')}{' '}
              {!demo && <code className="rounded bg-zinc-100 px-1.5 py-0.5 font-mono text-xs text-zinc-800 dark:bg-white/10 dark:text-white">sudo stacklane update</code>}
            </p>
          )}
          {error && <p className="text-sm font-medium text-red-500 dark:text-red-400">{error}</p>}
          <ConfirmDialog
            open={confirming}
            version={overview.latest}
            pending={start.isPending}
            onClose={() => setConfirming(false)}
            onConfirm={() => start.mutate(undefined, { onSettled: () => setConfirming(false) })}
          />
        </>
      )}
    </div>
  )
}
