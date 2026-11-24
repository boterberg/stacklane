import { intlLocale, t } from './i18n'

const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['week', 7 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
]

export function formatDate(value: string) {
  const formatted = new Intl.DateTimeFormat(intlLocale(), { day: '2-digit', month: '2-digit', year: 'numeric' }).format(new Date(value))
  return intlLocale() === 'nl-NL' ? formatted.replaceAll('/', '-') : formatted
}

export function formatRelative(value: string) {
  const formatter = new Intl.RelativeTimeFormat(intlLocale(), { numeric: 'auto' })
  const seconds = (new Date(value).getTime() - Date.now()) / 1000
  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) {
      return formatter.format(Math.round(seconds / size), unit)
    }
  }
  return t('zojuist')
}

export function formatEuro(cents: number) {
  return new Intl.NumberFormat(intlLocale(), { style: 'currency', currency: 'EUR' }).format(cents / 100)
}

function formatNumber(value: number, digits: number) {
  return new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: digits }).format(value)
}

const byteUnits = ['B', 'KB', 'MB', 'GB', 'TB']

export function formatBytes(bytes: number) {
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < byteUnits.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${formatNumber(value, value >= 10 || unit === 0 ? 0 : 1)} ${byteUnits[unit]}`
}

export function formatMegabytes(megabytes: number) {
  return megabytes >= 1024 ? `${formatNumber(megabytes / 1024, 2)} GB` : `${megabytes} MB`
}

export function formatDuration(milliseconds: number) {
  const minutes = Math.floor(milliseconds / 60_000)
  if (minutes < 60) return t('{minutes} min', { minutes })
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return t('{hours} u {minutes} min', { hours, minutes: minutes % 60 })
  return t('{days} d {hours} u', { days: Math.floor(hours / 24), hours: hours % 24 })
}
