import { ArrowLeftIcon, LifebuoyIcon } from '@heroicons/react/20/solid'
import { Link, useParams } from 'react-router'

import { ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useArticle } from '../../lib/kb'

export function ArticlePage() {
  const { slug = '' } = useParams()
  const { data: article, isPending, error } = useArticle(slug)

  if (isPending) return null
  if (error || !article) {
    return (
      <EmptyState
        title={t('Artikel niet gevonden')}
        description={t('Dit artikel bestaat niet of is offline gehaald.')}
        action={
          <ButtonLink to="/help" size="sm">
            {t('Naar het helpcentrum')}
          </ButtonLink>
        }
      />
    )
  }

  return (
    <article className="max-w-3xl">
      <Link to="/help" className="inline-flex items-center gap-1 text-sm text-zinc-500 hover:text-zinc-800 dark:text-white/60 dark:hover:text-white">
        <ArrowLeftIcon className="size-4" aria-hidden />
        {t('Helpcentrum')}
      </Link>
      <h1 className="mt-4 text-xl font-medium text-zinc-800 dark:text-white">{article.title}</h1>
      <p className="mt-1 text-sm text-zinc-500 dark:text-white/60">
        {article.category ? `${article.category} · ` : ''}
        {t('Bijgewerkt op {date}', { date: formatDate(article.updated_at) })}
      </p>
      <div className="rich-text mt-6 text-sm leading-6 text-zinc-700 dark:text-zinc-200" dangerouslySetInnerHTML={{ __html: article.body_html }} />
      <div className="mt-10 flex flex-wrap items-center justify-between gap-4 rounded-xl border border-zinc-200 bg-white p-5 dark:border-white/10 dark:bg-white/5">
        <div>
          <p className="text-sm font-medium text-zinc-800 dark:text-white">{t('Kom je er niet uit?')}</p>
          <p className="mt-1 text-sm text-zinc-500 dark:text-white/70">{t('Stel je vraag aan support, we helpen je graag.')}</p>
        </div>
        <ButtonLink to="/support/tickets?new=1" size="sm" icon={LifebuoyIcon}>
          {t('Ticket openen')}
        </ButtonLink>
      </div>
    </article>
  )
}
