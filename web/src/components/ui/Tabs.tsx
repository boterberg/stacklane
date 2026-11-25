import { cn } from '../../lib/cn'
import { t } from '../../lib/i18n'

type TabsProps<T extends string> = {
  tabs: Array<{ id: T; label: string }>
  active: T
  onChange: (id: T) => void
}

export function Tabs<T extends string>({ tabs, active, onChange }: TabsProps<T>) {
  return (
    <div className="-mx-6 mb-8 border-b border-zinc-200 px-6 sm:-mx-8 sm:px-8 dark:border-white/10">
      <nav className="-mb-px flex gap-2 overflow-x-auto" role="tablist">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={tab.id === active}
            onClick={() => onChange(tab.id)}
            className={cn(
              'shrink-0 border-b-2 px-3 py-3 text-sm font-medium whitespace-nowrap transition-colors',
              tab.id === active
                ? 'accent-border border-zinc-800 text-zinc-800 dark:border-white dark:text-white'
                : 'border-transparent text-zinc-500 hover:text-zinc-800 dark:text-white/70 dark:hover:text-white',
            )}
          >
            {t(tab.label)}
          </button>
        ))}
      </nav>
    </div>
  )
}
