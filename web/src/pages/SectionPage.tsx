import { EmptyState } from '../components/ui/EmptyState'
import { PageHeader } from '../components/ui/Heading'
import { t } from '../lib/i18n'

export function SectionPage({ title }: { title: string }) {
  return (
    <>
      <PageHeader title={t(title)} />
      <EmptyState title={t('Nog niet beschikbaar')} description={t('Dit onderdeel wordt op dit moment gebouwd.')} />
    </>
  )
}

export function NotFoundPage() {
  return (
    <>
      <PageHeader title={t('Pagina niet gevonden')} />
      <EmptyState title={t('Deze pagina bestaat niet')} description={t('Controleer het adres of ga terug naar het dashboard.')} />
    </>
  )
}
