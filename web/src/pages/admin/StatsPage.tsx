import { useState } from 'react'

import { Card } from '../../components/ui/Card'
import { PageHeader } from '../../components/ui/Heading'
import { useStats, type Stats } from '../../lib/admin'
import { formatMoney } from '../../lib/format'
import { intlLocale, t } from '../../lib/i18n'
import { serviceKindLabels } from '../../lib/labels'

function Tile({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="rounded-xl border border-zinc-200 bg-white p-5 dark:border-white/10 dark:bg-white/5">
      <p className="text-sm text-zinc-500 dark:text-white/70">{label}</p>
      <p className="mt-2 text-2xl font-medium tracking-tight text-zinc-800 tabular-nums dark:text-white">{value}</p>
      {detail && <p className="mt-1 text-xs text-zinc-500 dark:text-white/60">{detail}</p>}
    </div>
  )
}

function monthLabel(month: string, style: 'short' | 'long') {
  const [year, number] = month.split('-').map(Number)
  return new Intl.DateTimeFormat(intlLocale(), style === 'short' ? { month: 'short' } : { month: 'long', year: 'numeric' }).format(new Date(year, number - 1, 1))
}

function niceMax(value: number) {
  if (value <= 0) return 100
  const magnitude = 10 ** Math.floor(Math.log10(value))
  const step = [1, 2, 2.5, 5, 10].find((factor) => factor * magnitude >= value) ?? 10
  return step * magnitude
}

function RevenueChart({ stats }: { stats: Stats }) {
  const [active, setActive] = useState<number | null>(null)
  const width = 720
  const height = 240
  const left = 56
  const bottom = 28
  const top = 12
  const plotHeight = height - top - bottom
  const max = niceMax(Math.max(...stats.months.map((month) => month.revenue_cents)))
  const slot = (width - left) / stats.months.length
  const bar = Math.min(32, slot * 0.6)
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((share) => share * max)
  const compact = (cents: number) => new Intl.NumberFormat(intlLocale(), { style: 'currency', currency: stats.currency, notation: 'compact', maximumFractionDigits: 1 }).format(cents / 100)
  const shown = active === null ? null : stats.months[active]

  return (
    <div className="-mx-2 overflow-x-auto px-2">
      <div className="relative min-w-[520px]">
        <svg viewBox={`0 0 ${width} ${height}`} className="h-auto w-full" role="img" aria-label={t('Omzet per maand')} onMouseLeave={() => setActive(null)}>
          {ticks.map((tick) => {
            const y = top + plotHeight - (tick / max) * plotHeight
            return (
              <g key={tick}>
                <line x1={left} x2={width} y1={y} y2={y} className="stroke-zinc-200 dark:stroke-white/10" strokeWidth={1} />
                <text x={left - 8} y={y + 4} textAnchor="end" className="fill-zinc-400 text-[11px] dark:fill-white/50">
                  {compact(tick)}
                </text>
              </g>
            )
          })}
          {stats.months.map((month, index) => {
            const value = (month.revenue_cents / max) * plotHeight
            const x = left + index * slot + (slot - bar) / 2
            const y = top + plotHeight - value
            const radius = Math.min(4, value / 2)
            const path = value > 0 ? `M${x},${top + plotHeight} V${y + radius} Q${x},${y} ${x + radius},${y} H${x + bar - radius} Q${x + bar},${y} ${x + bar},${y + radius} V${top + plotHeight} Z` : ''
            return (
              <g
                key={month.month}
                onMouseEnter={() => setActive(index)}
                onFocus={() => setActive(index)}
                tabIndex={0}
                aria-label={`${monthLabel(month.month, 'long')}: ${formatMoney(month.revenue_cents, stats.currency)}`}
              >
                <rect x={left + index * slot} y={top} width={slot} height={plotHeight + bottom} fill="transparent" />
                {path && <path d={path} className={active === null || active === index ? 'chart-bar' : 'chart-bar opacity-40'} />}
                <text x={left + index * slot + slot / 2} y={height - 8} textAnchor="middle" className="fill-zinc-500 text-[11px] dark:fill-white/60">
                  {monthLabel(month.month, 'short')}
                </text>
              </g>
            )
          })}
        </svg>
        {shown && active !== null && (
          <div
            className="pointer-events-none absolute top-0 z-10 -translate-x-1/2 rounded-lg whitespace-nowrap border border-zinc-200 bg-white px-3 py-2 text-xs shadow-md dark:border-white/10 dark:bg-zinc-800"
            style={{ left: `${((left + active * slot + slot / 2) / width) * 100}%` }}
          >
            <p className="font-medium text-zinc-800 dark:text-white">{monthLabel(shown.month, 'long')}</p>
            <p className="mt-1 text-zinc-500 tabular-nums dark:text-white/70">{formatMoney(shown.revenue_cents, stats.currency)}</p>
            <p className="text-zinc-500 dark:text-white/70">{t('{count} nieuwe klanten', { count: shown.new_customers })}</p>
          </div>
        )}
      </div>
    </div>
  )
}

export function StatsPage() {
  const { data: stats } = useStats()
  if (!stats) return <PageHeader title={t('Statistieken')} />
  const money = (cents: number) => formatMoney(cents, stats.currency)
  const change = stats.revenue_last_month_cents > 0 ? Math.round(((stats.revenue_this_month_cents - stats.revenue_last_month_cents) / stats.revenue_last_month_cents) * 100) : null

  return (
    <>
      <PageHeader title={t('Statistieken')} description={t('Omzet en klanten in één oogopslag. Bedragen zijn inclusief btw.')} />
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Tile label={t('Terugkerende omzet per maand')} value={money(stats.monthly_recurring_cents)} detail={t('Van alle actieve abonnementen')} />
        <Tile
          label={t('Ontvangen deze maand')}
          value={money(stats.revenue_this_month_cents)}
          detail={
            change === null
              ? t('Vorige maand: {amount}', { amount: money(stats.revenue_last_month_cents) })
              : t('{change}% ten opzichte van vorige maand', { change: change > 0 ? `+${change}` : change })
          }
        />
        <Tile label={t('Openstaand')} value={money(stats.open_cents)} detail={stats.open_invoices === 1 ? t('1 factuur') : t('{count} facturen', { count: stats.open_invoices })} />
        <Tile label={t('Klanten')} value={String(stats.customers)} detail={t('{paying} betalend, {new} nieuw in 30 dagen', { paying: stats.paying_customers, new: stats.new_customers_30_days })} />
      </div>
      <div className="mt-6 grid grid-cols-1 gap-6 xl:grid-cols-3 xl:items-start">
        <Card title={t('Omzet per maand')} description={t('Betaalde facturen van de laatste 12 maanden.')} className="xl:col-span-2">
          <RevenueChart stats={stats} />
        </Card>
        <Card title={t('Actieve diensten')}>
          {stats.services.length === 0 ? (
            <p className="text-sm text-zinc-500 dark:text-white/70">{t('Er zijn nog geen actieve diensten.')}</p>
          ) : (
            <ul className="divide-y divide-zinc-200 dark:divide-white/10">
              {stats.services.map((service) => (
                <li key={service.kind} className="flex justify-between py-3 text-sm first:pt-0 last:pb-0">
                  <span className="text-zinc-800 dark:text-white">{serviceKindLabels[service.kind] ?? service.kind}</span>
                  <span className="text-zinc-500 tabular-nums dark:text-white/70">{service.count}</span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </>
  )
}
