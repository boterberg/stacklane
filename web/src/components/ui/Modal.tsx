import { Dialog, DialogBackdrop, DialogPanel, DialogTitle } from '@headlessui/react'
import { XMarkIcon } from '@heroicons/react/24/outline'
import type { ReactNode } from 'react'

import { t } from '../../lib/i18n'

type ModalProps = {
  open: boolean
  title: string
  description?: ReactNode
  footer: ReactNode
  children: ReactNode
  onClose: () => void
}

export function Modal({ open, title, description, footer, children, onClose }: ModalProps) {
  return (
    <Dialog open={open} onClose={onClose} className="relative z-50">
      <DialogBackdrop transition className="fixed inset-0 bg-zinc-900/20 transition-opacity data-closed:opacity-0 dark:bg-black/40" />
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <DialogPanel
          transition
          className="w-full max-w-sm overflow-hidden rounded-xl border border-zinc-200 bg-white shadow-xl transition data-closed:scale-95 data-closed:opacity-0 dark:border-white/10 dark:bg-zinc-800"
        >
          <div className="flex items-start justify-between gap-4 px-5 pt-5">
            <div>
              <DialogTitle className="text-base font-medium text-zinc-800 dark:text-white">{title}</DialogTitle>
              {description && <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{description}</p>}
            </div>
            <button type="button" onClick={onClose} className="rounded-md p-1 text-zinc-400 hover:text-zinc-600 dark:text-white/50 dark:hover:text-white" aria-label={t('Sluiten')}>
              <XMarkIcon className="size-5" aria-hidden />
            </button>
          </div>
          <div className="px-5 py-5">{children}</div>
          <div className="flex items-center justify-between gap-4 border-t border-zinc-200 bg-zinc-50 px-5 py-3 dark:border-white/10 dark:bg-white/5">{footer}</div>
        </DialogPanel>
      </div>
    </Dialog>
  )
}
