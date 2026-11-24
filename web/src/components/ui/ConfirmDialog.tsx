import { Dialog, DialogBackdrop, DialogPanel, DialogTitle } from '@headlessui/react'
import type { ReactNode } from 'react'

import { t } from '../../lib/i18n'
import { Button } from './Button'

type ConfirmDialogProps = {
  open: boolean
  title: string
  children: ReactNode
  confirmLabel: string
  danger?: boolean
  pending?: boolean
  onClose: () => void
  onConfirm: () => void
}

export function ConfirmDialog({ open, title, children, confirmLabel, danger, pending, onClose, onConfirm }: ConfirmDialogProps) {
  return (
    <Dialog open={open} onClose={onClose} className="relative z-50">
      <DialogBackdrop transition className="fixed inset-0 bg-zinc-900/20 transition-opacity data-closed:opacity-0 dark:bg-black/40" />
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <DialogPanel transition className="w-full max-w-md rounded-xl border border-zinc-200 bg-white p-6 shadow-xl transition data-closed:scale-95 data-closed:opacity-0 dark:border-white/10 dark:bg-zinc-800">
          <DialogTitle className="text-base font-medium text-zinc-800 dark:text-white">{title}</DialogTitle>
          <div className="mt-2 text-sm text-zinc-500 dark:text-white/70">{children}</div>
          <div className="mt-6 flex justify-end gap-2">
            <Button onClick={onClose}>{t('Annuleren')}</Button>
            <Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm} disabled={pending}>
              {confirmLabel}
            </Button>
          </div>
        </DialogPanel>
      </div>
    </Dialog>
  )
}
