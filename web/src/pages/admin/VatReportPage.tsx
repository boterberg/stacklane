import { ArrowDownTrayIcon } from '@heroicons/react/24/outline'
import { useState } from 'react'

import { EmptyState } from '../../components/ui/EmptyState'
import { Select } from '../../components/ui/Field'
import { PageHeader } from '../../components/ui/Heading'
import { ListToolbar, ToolbarSpacer } from '../../components/ui/ListToolbar'
import { DataTable } from '../../components/ui/ResourceList'
import { useVatReport, type VatRow } from '../../lib/admin'
import { countries } from '../../lib/countries'
import { formatMoney } from '../../lib/format'
import { intlLocale, t } from '../../lib/i18n'

function schemeLabel(scheme: VatRow['scheme']) {
  return { domestic: t('Binnenland'), oss: t('OSS'), reverse_charge: t('Btw verlegd'), outside_eu: t('Buiten de EU') }[scheme]
}

function sum(rows: VatRow[], key: 'invoices' | 'net_cents' | 'vat_cents' | 'gross_cents') {
  return rows.reduce((total, row) => total + row[key], 0)
}

const today = new Date()
const thisYear = today.getFullYear()
const thisQuarter = Math.floor(today.getMonth() / 3) + 1

export function VatReportPage() {
  const [year, setYear] = useState(thisYear)
  const [quarter, setQuarter] = useState(thisQuarter)
  const { data: rows = [], isPending } = useVatReport(year, quarter)
  const names = new Map(countries().map((country) => [country.code, country.name]))
  const oss = rows.filter((row) => row.scheme === 'oss')
  const years = Array.from({ length: 5 }, (_, index) => thisYear - index)

  return (
    <>
      <PageHeader title={t('Btw-overzicht')} description={t('Omzet en btw per regeling en land, op factuurdatum. De regels met OSS gaan in je OSS-aangifte.')} />
      <ListToolbar>
        <div className="w-32">
          <Select aria-label={t('Jaar')} value={year} onChange={(event) => setYear(Number(event.target.value))}>
            {years.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </Select>
        </div>
        <div className="w-40">
          <Select aria-label={t('Kwartaal')} value={quarter} onChange={(event) => setQuarter(Number(event.target.value))}>
            {[1, 2, 3, 4].map((option) => (
              <option key={option} value={option}>
                {t('Kwartaal {number}', { number: option })}
              </option>
            ))}
          </Select>
        </div>
        <ToolbarSpacer />
        <div className="flex gap-2">
          <a
            href={`/api/admin/reports/vat?year=${year}&quarter=${quarter}&format=oss`}
            className="inline-flex h-10 items-center gap-2 rounded-lg border border-zinc-200 border-b-zinc-300/80 bg-white px-4 text-sm font-medium text-zinc-800 shadow-xs hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75"
          >
            <ArrowDownTrayIcon className="-ml-0.5 size-4" aria-hidden />
            {t('OSS-aangifte downloaden')}
          </a>
          <a
            href={`/api/admin/reports/vat?year=${year}&quarter=${quarter}&format=csv`}
            className="inline-flex h-10 items-center gap-2 rounded-lg border border-zinc-200 border-b-zinc-300/80 bg-white px-4 text-sm font-medium text-zinc-800 shadow-xs hover:bg-zinc-50 dark:border-zinc-600 dark:bg-zinc-700 dark:text-white dark:hover:bg-zinc-600/75"
          >
            <ArrowDownTrayIcon className="-ml-0.5 size-4" aria-hidden />
            {t('CSV downloaden')}
          </a>
        </div>
      </ListToolbar>
      {!isPending && rows.length === 0 ? (
        <EmptyState title={t('Geen facturen')} description={t('In dit kwartaal zijn geen facturen gemaakt.')} />
      ) : (
        <>
          <DataTable<VatRow>
            rows={rows}
            rowKey={(row) => `${row.scheme}-${row.country}-${row.rate_basis_points}`}
            columns={[
              { header: t('Regeling'), cell: (row) => schemeLabel(row.scheme) },
              { header: t('Land'), cell: (row) => (row.country ? (names.get(row.country) ?? row.country) : '-') },
              { header: t('Tarief'), cell: (row) => `${(row.rate_basis_points / 100).toLocaleString(intlLocale())}%` },
              { header: t('Facturen'), cell: (row) => row.invoices },
              { header: t('Zonder btw'), cell: (row) => formatMoney(row.net_cents) },
              { header: t('Btw'), cell: (row) => formatMoney(row.vat_cents) },
              { header: t('Totaal'), cell: (row) => formatMoney(row.gross_cents) },
            ]}
          />
          {oss.length > 0 && (
            <p className="mt-6 text-sm text-zinc-500 dark:text-white/70">
              {t('Voor je OSS-aangifte: {net} omzet en {vat} btw over {count} landen.', {
                net: formatMoney(sum(oss, 'net_cents')),
                vat: formatMoney(sum(oss, 'vat_cents')),
                count: new Set(oss.map((row) => row.country)).size,
              })}
            </p>
          )}
        </>
      )}
    </>
  )
}
