import { ChevronRightIcon } from '@heroicons/react/20/solid'
import { useState } from 'react'
import { Link } from 'react-router'

import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField } from '../../components/ui/ListToolbar'
import { groupByCategory, useArticles } from '../../lib/kb'
import { t } from '../../lib/i18n'

export function HelpPage() {
  const [search, setSearch] = useState('')
  const { data: articles, isPending } = useArticles(search)
  const groups = groupByCategory(articles ?? [], t('Algemeen'))

  return (
    <>
      <PageHeader title={t('Helpcentrum')} description={t('Handleidingen en antwoorden op veelgestelde vragen.')} />
      <ListToolbar>
        <SearchField value={search} onChange={setSearch} placeholder={t('Zoek in het helpcentrum...')} />
      </ListToolbar>
      {!isPending && groups.length === 0 ? (
        <EmptyState
          title={search ? t('Niets gevonden') : t('Nog geen artikelen')}
          description={search ? t('Probeer een ander woord, of stel je vraag in een ticket.') : t('Er staan nog geen artikelen in het helpcentrum. Stel je vraag gerust in een ticket.')}
        />
      ) : (
        <div className="grid max-w-5xl grid-cols-1 gap-6 lg:grid-cols-2 lg:items-start">
          {groups.map(([category, items]) => (
            <section key={category} className="rounded-xl border border-zinc-200 bg-white dark:border-white/10 dark:bg-white/5">
              <h2 className="border-b border-zinc-200 px-5 py-3 text-sm font-medium text-zinc-800 dark:border-white/10 dark:text-white">{category}</h2>
              <ul className="divide-y divide-zinc-200 dark:divide-white/10">
                {items.map((article) => (
                  <li key={article.id}>
                    <Link to={`/help/${article.slug}`} className="group flex items-center gap-3 px-5 py-3 transition-colors hover:bg-zinc-50 dark:hover:bg-white/5">
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-zinc-800 dark:text-white">{article.title}</p>
                        <p className="mt-0.5 line-clamp-2 text-sm text-zinc-500 dark:text-white/60">{article.excerpt}</p>
                      </div>
                      <ChevronRightIcon className="size-5 shrink-0 text-zinc-400 transition-transform group-hover:translate-x-0.5 dark:text-white/40" aria-hidden />
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      )}
    </>
  )
}
