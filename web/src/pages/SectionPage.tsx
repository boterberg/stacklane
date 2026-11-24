import { EmptyState } from '../components/ui/EmptyState'
import { PageHeader } from '../components/ui/Heading'

export function SectionPage({ title }: { title: string }) {
  return (
    <>
      <PageHeader title={title} />
      <EmptyState title="Nog niet beschikbaar" description="Dit onderdeel wordt op dit moment gebouwd." />
    </>
  )
}

export function NotFoundPage() {
  return (
    <>
      <PageHeader title="Pagina niet gevonden" />
      <EmptyState title="Deze pagina bestaat niet" description="Controleer het adres of ga terug naar het dashboard." />
    </>
  )
}
