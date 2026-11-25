import { CalendarIcon, FolderIcon, PlusIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { Badge } from '../../components/ui/Badge'
import { ButtonLink } from '../../components/ui/Button'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, SearchField } from '../../components/ui/ListToolbar'
import { RowCard, RowCardList } from '../../components/ui/ResourceList'
import { formatDate } from '../../lib/format'
import { t } from '../../lib/i18n'
import { useAdminArticles } from '../../lib/kb'

export function KbPage() {
  const [search, setSearch] = useState('')
  const { data: articles, isPending } = useAdminArticles(search)
  const list = articles ?? []

  return (
    <>
      <PageHeader
        title={t('Kennisbank')}
        description={t('Artikelen voor het helpcentrum van je klanten. Alleen gepubliceerde artikelen zijn zichtbaar.')}
        actions={
          <ButtonLink to="/admin/kb/new" size="sm" icon={PlusIcon}>
            {t('Artikel schrijven')}
          </ButtonLink>
        }
      />
      <ListToolbar>
        <SearchField value={search} onChange={setSearch} placeholder={t('Zoek op titel of tekst...')} />
      </ListToolbar>
      {!isPending && list.length === 0 ? (
        <EmptyState title={t('Nog geen artikelen')} description={t('Schrijf je eerste artikel, bijvoorbeeld over inloggen op een VPS of het betalen van een factuur.')} />
      ) : (
        <RowCardList>
          {list.map((article) => (
            <RowCard
              key={article.id}
              to={`/admin/kb/${article.id}`}
              title={article.title}
              badge={article.published ? <Badge color="green">{t('Gepubliceerd')}</Badge> : <Badge color="zinc">{t('Concept')}</Badge>}
              meta={[
                { icon: FolderIcon, label: article.category || t('Algemeen') },
                { icon: CalendarIcon, label: t('Bijgewerkt op {date}', { date: formatDate(article.updated_at) }) },
              ]}
            />
          ))}
        </RowCardList>
      )}
    </>
  )
}
