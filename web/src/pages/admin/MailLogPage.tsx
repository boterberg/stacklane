import { useState } from 'react'

import { Badge, type BadgeColor } from '../../components/ui/Badge'
import { EmptyState } from '../../components/ui/EmptyState'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, MultiSelectFilter, PerPageSelect, SearchField, ToolbarSpacer } from '../../components/ui/ListToolbar'
import { DataTable, PaginationFooter } from '../../components/ui/ResourceList'
import { useMailLog, type MailEntry } from '../../lib/admin'
import { formatDate } from '../../lib/format'
import { intlLocale, t } from '../../lib/i18n'
import { useStoredState } from '../../lib/useStoredState'

const statuses = ['sent', 'failed', 'skipped']

function statusLabels(): Record<string, string> {
  return {
    sent: t('Verstuurd'),
    failed: t('Mislukt'),
    skipped: t('Niet verstuurd'),
  }
}

const statusColors: Record<MailEntry['status'], BadgeColor> = {
  sent: 'green',
  failed: 'red',
  skipped: 'zinc',
}

function kindLabel(kind: string) {
  const labels: Record<string, string> = {
    ticket_reply: t('Antwoord op ticket'),
    notification: t('Melding'),
    billing: t('Facturatie'),
    password_reset: t('Wachtwoord herstellen'),
    email_verification: t('E-mailadres bevestigen'),
    email_change_notice: t('E-mailadres gewijzigd'),
  }
  return labels[kind] ?? kind
}

function time(value: string) {
  return `${formatDate(value)} ${new Intl.DateTimeFormat(intlLocale(), { hour: '2-digit', minute: '2-digit' }).format(new Date(value))}`
}

export function MailLogPage() {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<string[]>([])
  const [page, setPage] = useState(1)
  const [perPage, setPerPage] = useStoredState<number>('stacklane-per-page-mail-log', 25)
  const { data, isPending } = useMailLog({
    q: query,
    statuses: filter,
    page,
    perPage,
  })
  const entries = data?.data ?? []
  const labels = statusLabels()

  function resetPage<T>(setter: (value: T) => void) {
    return (value: T) => {
      setter(value)
      setPage(1)
    }
  }

  return (
    <>
      <PageHeader title={t('Maillog')} description={t('Elke e-mail die Stacklane verstuurde, de laatste 90 dagen. De inhoud bewaren we niet, want daar staan soms inloglinks in.')} />
      <ListToolbar>
        <SearchField value={query} onChange={resetPage(setQuery)} placeholder={t('Zoek op ontvanger of onderwerp...')} />
        <ToolbarSpacer />
        <MultiSelectFilter values={filter} options={statuses} labels={labels} onChange={resetPage(setFilter)} noun={[t('status'), t('statussen')]} placeholder={t('Filter op status...')} />
        <PerPageSelect value={perPage} onChange={resetPage(setPerPage)} />
      </ListToolbar>
      {!isPending && entries.length === 0 ? (
        <EmptyState title={t('Geen e-mails')} description={t('Er zijn geen e-mails gevonden.')} />
      ) : (
        <DataTable
          rows={entries}
          rowKey={(entry) => entry.id}
          columns={[
            {
              header: t('Verstuurd op'),
              cell: (entry) => <span className="whitespace-nowrap">{time(entry.created_at)}</span>,
            },
            {
              header: t('Ontvanger'),
              cell: (entry) => <span className="break-all">{entry.recipient}</span>,
            },
            {
              header: t('Onderwerp'),
              cell: (entry) => (
                <div>
                  <div className="text-zinc-800 dark:text-white">{entry.subject}</div>
                  <div className="text-xs text-zinc-500 dark:text-white/60">{kindLabel(entry.kind)}</div>
                  {entry.error && <div className="mt-1 text-xs break-all text-red-600 dark:text-red-400">{entry.error}</div>}
                </div>
              ),
            },
            {
              header: t('Status'),
              cell: (entry) => <Badge color={statusColors[entry.status]}>{labels[entry.status]}</Badge>,
            },
          ]}
        />
      )}
      {data && entries.length > 0 && <PaginationFooter meta={data.meta} onPage={setPage} />}
    </>
  )
}
