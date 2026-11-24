import { intlLocale } from './i18n'

const codes = ['NL', 'BE', 'DE', 'FR', 'LU', 'GB', 'IE', 'AT', 'CH', 'DK', 'SE', 'NO', 'FI', 'ES', 'PT', 'IT', 'PL', 'CZ', 'US', 'CA']

export function countries(): Array<{ code: string; name: string }> {
  const names = new Intl.DisplayNames([intlLocale()], { type: 'region' })
  return codes.map((code) => ({ code, name: names.of(code) ?? code }))
}
