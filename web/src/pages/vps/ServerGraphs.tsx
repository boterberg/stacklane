import { useQuery } from '@tanstack/react-query'
import { useId, useState, type PointerEvent } from 'react'

import { Card } from '../../components/ui/Card'
import { Select } from '../../components/ui/Field'
import { api } from '../../lib/api'
import { formatBytes } from '../../lib/format'
import { intlLocale, t } from '../../lib/i18n'

type Metric = {
  time: number
  cpu: number | null
  memory: number | null
  max_memory: number | null
  net_in: number | null
  net_out: number | null
  disk_read: number | null
  disk_write: number | null
}

type Series = { label: string; stroke: string; swatch: string; values: Array<number | null> }

const timeframes = [
  { value: 'hour', label: 'Laatste uur' },
  { value: 'day', label: 'Laatste dag' },
  { value: 'week', label: 'Laatste week' },
  { value: 'month', label: 'Laatste maand' },
]

const first = { stroke: 'stroke-[#2a78d6] dark:stroke-[#3987e5]', swatch: 'bg-[#2a78d6] dark:bg-[#3987e5]' }
const second = { stroke: 'stroke-[#eb6834] dark:stroke-[#d95926]', swatch: 'bg-[#eb6834] dark:bg-[#d95926]' }

const width = 760
const height = 200
const pad = { top: 12, right: 12, bottom: 24, left: 64 }

function timeLabel(seconds: number, timeframe: string) {
  const date = new Date(seconds * 1000)
  const options: Intl.DateTimeFormatOptions =
    timeframe === 'hour' || timeframe === 'day'
      ? { hour: '2-digit', minute: '2-digit', timeZone: 'Europe/Amsterdam' }
      : { day: 'numeric', month: 'short', timeZone: 'Europe/Amsterdam' }
  return date.toLocaleString(intlLocale(), options)
}

function niceMax(value: number) {
  if (value <= 0) return 1
  const power = 10 ** Math.floor(Math.log10(value))
  const step = [1, 2, 2.5, 5, 10].find((candidate) => candidate * power >= value) ?? 10
  return step * power
}

function LineChart({ title, points, series, format, timeframe, fixedMax }: { title: string; points: Metric[]; series: Series[]; format: (value: number) => string; timeframe: string; fixedMax?: number }) {
  const [hover, setHover] = useState<number | null>(null)
  const titleId = useId()
  const values = series.flatMap((item) => item.values.filter((value): value is number => value !== null))
  const max = fixedMax ?? niceMax(Math.max(0, ...values))
  const innerWidth = width - pad.left - pad.right
  const innerHeight = height - pad.top - pad.bottom
  const x = (index: number) => pad.left + (points.length <= 1 ? 0 : (index / (points.length - 1)) * innerWidth)
  const y = (value: number) => pad.top + innerHeight - (Math.min(value, max) / max) * innerHeight
  const ticks = [0, max / 2, max]
  const latest = series.map((item) => [...item.values].reverse().find((value) => value !== null) ?? null)

  function path(values: Array<number | null>) {
    let line = ''
    let open = false
    values.forEach((value, index) => {
      if (value === null) {
        open = false
        return
      }
      line += `${open ? 'L' : 'M'}${x(index).toFixed(1)},${y(value).toFixed(1)}`
      open = true
    })
    return line
  }

  function track(event: PointerEvent<SVGSVGElement>) {
    if (points.length === 0) return
    const box = event.currentTarget.getBoundingClientRect()
    const position = ((event.clientX - box.left) / box.width) * width
    const index = Math.round(((position - pad.left) / innerWidth) * (points.length - 1))
    setHover(Math.max(0, Math.min(points.length - 1, index)))
  }

  const tooltipLeft = hover === null ? 0 : (x(hover) / width) * 100

  return (
    <Card
      title={<span id={titleId}>{title}</span>}
      action={
        series.length > 1 ? (
          <div className="flex flex-wrap gap-4 text-xs text-zinc-500 dark:text-white/70">
            {series.map((item, index) => (
              <span key={item.label} className="flex items-center gap-1.5">
                <span className={`h-0.5 w-3 rounded-full ${item.swatch}`} aria-hidden />
                {item.label}
                {latest[index] !== null && <span className="text-zinc-800 tabular-nums dark:text-white">{format(latest[index] ?? 0)}</span>}
              </span>
            ))}
          </div>
        ) : (
          latest[0] !== null && <span className="text-sm text-zinc-800 tabular-nums dark:text-white">{format(latest[0] ?? 0)}</span>
        )
      }
    >
      {points.length === 0 ? (
        <p className="text-sm text-zinc-500 dark:text-white/60">{t('Nog geen gegevens voor deze periode.')}</p>
      ) : (
        <div className="relative">
          <svg viewBox={`0 0 ${width} ${height}`} className="h-auto w-full touch-none" role="img" aria-labelledby={titleId} onPointerMove={track} onPointerLeave={() => setHover(null)}>
            {ticks.map((tick) => (
              <g key={tick}>
                <line x1={pad.left} x2={width - pad.right} y1={y(tick)} y2={y(tick)} className="stroke-zinc-200 dark:stroke-white/10" strokeWidth={1} />
                <text x={pad.left - 8} y={y(tick)} textAnchor="end" dominantBaseline="middle" className="fill-zinc-400 text-[12px] dark:fill-white/40">
                  {format(tick)}
                </text>
              </g>
            ))}
            {[0, Math.floor((points.length - 1) / 2), points.length - 1].map((index) => (
              <text key={index} x={x(index)} y={height - 6} textAnchor={index === 0 ? 'start' : index === points.length - 1 ? 'end' : 'middle'} className="fill-zinc-400 text-[12px] dark:fill-white/40">
                {timeLabel(points[index].time, timeframe)}
              </text>
            ))}
            {series.map((item) => (
              <path key={item.label} d={path(item.values)} fill="none" strokeWidth={2} strokeLinejoin="round" strokeLinecap="round" className={item.stroke} />
            ))}
            {hover !== null && (
              <g>
                <line x1={x(hover)} x2={x(hover)} y1={pad.top} y2={pad.top + innerHeight} className="stroke-zinc-400 dark:stroke-white/40" strokeWidth={1} />
                {series.map((item) =>
                  item.values[hover] === null ? null : (
                    <circle key={item.label} cx={x(hover)} cy={y(item.values[hover] ?? 0)} r={4} strokeWidth={2} className={`fill-white dark:fill-zinc-800 ${item.stroke}`} />
                  ),
                )}
              </g>
            )}
          </svg>
          {hover !== null && (
            <div
              className="pointer-events-none absolute top-0 z-10 min-w-36 -translate-x-1/2 rounded-lg border border-zinc-200 bg-white px-3 py-2 text-xs shadow-lg dark:border-white/10 dark:bg-zinc-800"
              style={{ left: `${Math.min(Math.max(tooltipLeft, 12), 88)}%` }}
            >
              <p className="mb-1 text-zinc-500 dark:text-white/60">{timeLabel(points[hover].time, 'hour')}</p>
              {series.map((item) => (
                <p key={item.label} className="flex items-center justify-between gap-4 text-zinc-800 dark:text-white">
                  <span className="flex items-center gap-1.5">
                    <span className={`h-0.5 w-3 rounded-full ${item.swatch}`} aria-hidden />
                    {item.label}
                  </span>
                  <span className="tabular-nums">{item.values[hover] === null ? '-' : format(item.values[hover] ?? 0)}</span>
                </p>
              ))}
            </div>
          )}
        </div>
      )}
    </Card>
  )
}

const percent = (value: number) => `${Math.round(value)}%`
const rate = (value: number) => `${formatBytes(value)}/s`

export function ServerGraphs({ serverId }: { serverId: string }) {
  const [timeframe, setTimeframe] = useState('hour')
  const { data: points = [], isPending } = useQuery({
    queryKey: ['vps', 'graphs', serverId, timeframe],
    queryFn: () => api<Metric[]>(`/vps/servers/${serverId}/graphs?timeframe=${timeframe}`),
    refetchInterval: timeframe === 'hour' ? 60_000 : false,
  })
  const pick = (key: keyof Metric) => points.map((point) => point[key])
  const maxMemory = Math.max(0, ...points.map((point) => point.max_memory ?? 0))

  return (
    <div className="max-w-4xl space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="text-sm text-zinc-500 dark:text-white/70">{t('Bekijk het CPU-, geheugen-, schijf- en netwerkgebruik van je server. Tijden worden getoond in Europe/Amsterdam.')}</p>
        <div className="w-44">
          <Select aria-label={t('Periode')} value={timeframe} onChange={(event) => setTimeframe(event.target.value)}>
            {timeframes.map((item) => (
              <option key={item.value} value={item.value}>
                {t(item.label)}
              </option>
            ))}
          </Select>
        </div>
      </div>
      {!isPending && (
        <>
          <LineChart
            title={t('CPU-gebruik')}
            timeframe={timeframe}
            points={points}
            format={percent}
            fixedMax={100}
            series={[{ label: t('CPU'), ...first, values: pick('cpu').map((value) => (value === null ? null : (value as number) * 100)) }]}
          />
          <LineChart
            title={t('Geheugengebruik')}
            timeframe={timeframe}
            points={points}
            format={formatBytes}
            fixedMax={maxMemory > 0 ? maxMemory : undefined}
            series={[{ label: t('Geheugen'), ...first, values: pick('memory') as Array<number | null> }]}
          />
          <LineChart
            title={t('Netwerk')}
            timeframe={timeframe}
            points={points}
            format={rate}
            series={[
              { label: t('Inkomend'), ...first, values: pick('net_in') as Array<number | null> },
              { label: t('Uitgaand'), ...second, values: pick('net_out') as Array<number | null> },
            ]}
          />
          <LineChart
            title={t('Schijf')}
            timeframe={timeframe}
            points={points}
            format={rate}
            series={[
              { label: t('Lezen'), ...first, values: pick('disk_read') as Array<number | null> },
              { label: t('Schrijven'), ...second, values: pick('disk_write') as Array<number | null> },
            ]}
          />
        </>
      )}
    </div>
  )
}
