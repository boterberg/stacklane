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
