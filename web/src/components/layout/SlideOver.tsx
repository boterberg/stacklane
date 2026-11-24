import { Dialog, DialogBackdrop, DialogPanel, DialogTitle } from '@headlessui/react'
import { XMarkIcon } from '@heroicons/react/24/outline'
import type { ReactNode } from 'react'

type SlideOverProps = {
  open: boolean
  onClose: () => void
  title: string
  description?: ReactNode
  footer?: ReactNode
  children: ReactNode
}

export function SlideOver({ open, onClose, title, description, footer, children }: SlideOverProps) {
  return (
    <Dialog open={open} onClose={onClose} className="relative z-50">
      <DialogBackdrop transition className="fixed inset-0 bg-zinc-900/20 transition-opacity duration-200 data-closed:opacity-0 dark:bg-black/40" />
      <div className="fixed inset-y-0 right-0 flex max-w-full pl-10 lg:py-2 lg:pr-2">
        <DialogPanel
          transition
          className="flex w-screen max-w-2xl flex-col border-zinc-200 bg-white shadow-xl transition duration-200 data-closed:translate-x-full lg:rounded-xl lg:border dark:border-white/10 dark:bg-zinc-800"
        >
          <div className="flex items-start justify-between gap-4 px-8 pt-8 pb-6">
            <div>
              <DialogTitle className="text-base font-medium text-zinc-800 dark:text-white">{title}</DialogTitle>
              {description && <p className="mt-2 text-sm text-zinc-500 dark:text-white/70">{description}</p>}
            </div>
            <button type="button" onClick={onClose} className="rounded-md p-1 text-zinc-400 hover:text-zinc-800 dark:hover:text-white" aria-label="Sluiten">
              <XMarkIcon className="size-5" />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto px-8 pb-8">{children}</div>
          {footer && <div className="flex justify-end gap-2 border-t border-zinc-200 bg-zinc-50 px-8 py-4 lg:rounded-b-xl dark:border-white/10 dark:bg-zinc-900/40">{footer}</div>}
        </DialogPanel>
      </div>
    </Dialog>
  )
}
