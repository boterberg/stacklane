export type Neutral = 'zinc' | 'slate' | 'gray' | 'neutral' | 'stone' | 'tinted'
export type Radius = 'default' | 'sharp' | 'round'

const steps = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950]

const palettes: Record<Exclude<Neutral, 'zinc' | 'tinted'>, string[]> = {
  slate: ['98.4% 0.003 247.858', '96.8% 0.007 247.896', '92.9% 0.013 255.508', '86.9% 0.022 252.894', '70.4% 0.04 256.788', '55.4% 0.046 257.417', '44.6% 0.043 257.281', '37.2% 0.044 257.287', '27.9% 0.041 260.031', '20.8% 0.042 265.755', '12.9% 0.042 264.695'],
  gray: ['98.5% 0.002 247.839', '96.7% 0.003 264.542', '92.8% 0.006 264.531', '87.2% 0.01 258.338', '70.7% 0.022 261.325', '55.1% 0.027 264.364', '44.6% 0.03 256.802', '37.3% 0.034 259.733', '27.8% 0.033 256.848', '21% 0.034 264.665', '13% 0.028 261.692'],
  neutral: ['98.5% 0 0', '97% 0 0', '92.2% 0 0', '87% 0 0', '70.8% 0 0', '55.6% 0 0', '43.9% 0 0', '37.1% 0 0', '26.9% 0 0', '20.5% 0 0', '14.5% 0 0'],
  stone: ['98.5% 0.001 106.423', '97% 0.001 106.424', '92.3% 0.003 48.717', '86.9% 0.005 56.366', '70.9% 0.01 56.259', '55.3% 0.013 58.071', '44.4% 0.011 73.639', '37.4% 0.01 67.558', '26.8% 0.007 34.298', '21.6% 0.006 56.043', '14.7% 0.004 49.25'],
}

const tintedSteps: [number, number][] = [
  [98.4, 0.004],
  [96.6, 0.008],
  [92.6, 0.014],
  [86.8, 0.022],
  [70.5, 0.035],
  [55.3, 0.04],
  [44.5, 0.038],
  [37.2, 0.036],
  [27.6, 0.032],
  [20.8, 0.03],
  [13.4, 0.026],
]

const radii: Record<Exclude<Radius, 'default'>, number[]> = {
  sharp: [0.0625, 0.125, 0.1875, 0.25, 0.375, 0.5, 0.75],
  round: [0.1875, 0.375, 0.5, 0.75, 1, 1.25, 1.75],
}
const radiusNames = ['xs', 'sm', 'md', 'lg', 'xl', '2xl', '3xl']

function linear(channel: number) {
  return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
}

export function hue(hex: string) {
  const [r, g, b] = [1, 3, 5].map((start) => linear(parseInt(hex.slice(start, start + 2), 16) / 255))
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s
  const bb = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
  return ((Math.atan2(bb, a) * 180) / Math.PI + 360) % 360
}

export function contrast(hex: string) {
  const [r, g, b] = [1, 3, 5].map((start) => linear(parseInt(hex.slice(start, start + 2), 16) / 255))
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? '#18181b' : '#ffffff'
}

function neutralValues(neutral: Neutral, accent: string | null) {
  if (neutral === 'tinted') {
    const angle = accent ? hue(accent).toFixed(1) : '264'
    return tintedSteps.map(([lightness, chroma]) => `${lightness}% ${chroma} ${angle}`)
  }
  return neutral === 'zinc' ? null : palettes[neutral]
}

export type ThemeInput = {
  accent: string | null
  accent_dark: string | null
  neutral: string | null
  radius: string | null
  css: string | null
}

export function applyTheme(theme: ThemeInput) {
  const root = document.documentElement
  const set = (name: string, value: string | null) => (value === null ? root.style.removeProperty(name) : root.style.setProperty(name, value))

  if (theme.accent) {
    const dark = theme.accent_dark ?? theme.accent
    root.dataset.accent = ''
    set('--accent-light', theme.accent)
    set('--accent-light-contrast', contrast(theme.accent))
    set('--accent-dark', dark)
    set('--accent-dark-contrast', contrast(dark))
  } else {
    delete root.dataset.accent
    for (const name of ['--accent-light', '--accent-light-contrast', '--accent-dark', '--accent-dark-contrast']) set(name, null)
  }

  const values = neutralValues((theme.neutral ?? 'zinc') as Neutral, theme.accent)
  steps.forEach((step, index) => set(`--color-zinc-${step}`, values ? `oklch(${values[index]})` : null))

  const scale = theme.radius && theme.radius !== 'default' ? radii[theme.radius as Exclude<Radius, 'default'>] : null
  radiusNames.forEach((name, index) => set(`--radius-${name}`, scale ? `${scale[index]}rem` : null))

  let link = document.getElementById('theme-css') as HTMLLinkElement | null
  if (theme.css) {
    if (!link) {
      link = document.createElement('link')
      link.id = 'theme-css'
      link.rel = 'stylesheet'
      document.head.appendChild(link)
    }
    const href = `/api/branding/theme.css?v=${theme.css}`
    if (link.getAttribute('href') !== href) link.href = href
  } else {
    link?.remove()
  }
}

const zinc = ['98.5% 0 0', '96.7% 0.001 286.375', '92% 0.004 286.32', '87.1% 0.006 286.286', '70.5% 0.015 286.067', '55.2% 0.016 285.938', '44.2% 0.017 285.786', '37% 0.013 285.805', '27.4% 0.006 286.033', '21% 0.006 285.885', '14.1% 0.005 285.823']

export function neutralSwatch(neutral: Neutral, accent: string | null) {
  const values = neutralValues(neutral, accent) ?? zinc
  return [1, 4, 8].map((index) => `oklch(${values[index]})`)
}

export const presets: { name: string; accent: string; accent_dark: string; neutral: Neutral }[] = [
  { name: 'Indigo', accent: '#4f46e5', accent_dark: '#818cf8', neutral: 'slate' },
  { name: 'Oceaan', accent: '#0369a1', accent_dark: '#38bdf8', neutral: 'tinted' },
  { name: 'Bos', accent: '#15803d', accent_dark: '#4ade80', neutral: 'stone' },
  { name: 'Zonsondergang', accent: '#c2410c', accent_dark: '#fb923c', neutral: 'stone' },
  { name: 'Framboos', accent: '#be123c', accent_dark: '#fb7185', neutral: 'neutral' },
  { name: 'Lavendel', accent: '#7c3aed', accent_dark: '#a78bfa', neutral: 'tinted' },
]
