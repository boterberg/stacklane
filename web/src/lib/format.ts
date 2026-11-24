const dateFormatter = new Intl.DateTimeFormat('nl-NL', { day: '2-digit', month: '2-digit', year: 'numeric' })
const relativeFormatter = new Intl.RelativeTimeFormat('nl-NL', { numeric: 'auto' })
const euroFormatter = new Intl.NumberFormat('nl-NL', { style: 'currency', currency: 'EUR' })

const units: Array<[Intl.RelativeTimeFormatUnit, number]> = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['week', 7 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
]

export function formatDate(value: string) {
  return dateFormatter.format(new Date(value)).replaceAll('/', '-')
}

export function formatRelative(value: string) {
  const seconds = (new Date(value).getTime() - Date.now()) / 1000
  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) {
      return relativeFormatter.format(Math.round(seconds / size), unit)
    }
  }
  return 'zojuist'
}

export function formatEuro(cents: number) {
  return euroFormatter.format(cents / 100)
}

const byteUnits = ['B', 'KB', 'MB', 'GB', 'TB']

export function formatBytes(bytes: number) {
  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < byteUnits.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value >= 10 || unit === 0 ? Math.round(value) : value.toFixed(1).replace('.', ',')} ${byteUnits[unit]}`
}

export function formatMegabytes(megabytes: number) {
  return megabytes >= 1024 ? `${(megabytes / 1024).toString().replace('.', ',')} GB` : `${megabytes} MB`
}

export function formatDuration(milliseconds: number) {
  const minutes = Math.floor(milliseconds / 60_000)
  if (minutes < 60) return `${minutes} min`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} u ${minutes % 60} min`
  return `${Math.floor(hours / 24)} d ${hours % 24} u`
}
