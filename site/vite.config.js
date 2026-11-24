import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

const root = fileURLToPath(new URL('.', import.meta.url))
const version = process.env.STACKLANE_VERSION ?? 'dev'

function icon(name, className) {
  const svg = readFileSync(`${root}node_modules/heroicons/24/outline/${name}.svg`, 'utf8').trim()
  return svg.replace('<svg ', `<svg class="${className}" `)
}

function logo(className) {
  const source = readFileSync(`${root}../logo/stacklane-horizontal.svg`, 'utf8')
  const viewBox = source.match(/viewBox="([^"]+)"/)[1]
  const [symbol, ...wordmark] = [...source.matchAll(/<path fill="[^"]+" d="([^"]+)"\/>/g)].map((match) => match[1])
  return `<svg viewBox="${viewBox}" class="${className}" role="img" aria-label="Stacklane"><path fill="#4F46E5" d="${symbol}"/><path fill="currentColor" d="${wordmark.join(' ')}"/></svg>`
}

function feature(name, title, text) {
  return `<div class="rounded-xl border border-zinc-200 bg-white p-6 dark:border-white/10 dark:bg-white/5">
              ${icon(name, 'size-6 text-brand')}
              <h3 class="mt-4 text-base font-medium text-zinc-900 dark:text-white">${title}</h3>
              <p class="mt-2 text-sm text-zinc-500 dark:text-white/70">${text}</p>
            </div>`
}

function spec(name, title, text) {
  return `<div>
              <dt class="flex items-center gap-3 text-sm font-medium text-zinc-900 dark:text-white">${icon(name, 'size-5 text-brand')}${title}</dt>
              <dd class="mt-2 text-sm text-zinc-500 dark:text-white/70">${text}</dd>
            </div>`
}

const expandPlaceholders = {
  name: 'stacklane-placeholders',
  transformIndexHtml(html) {
    return html
      .replaceAll('{{version}}', version)
      .replace(/\{\{logo:([^}]+)\}\}/g, (_, className) => logo(className))
      .replace(/\{\{icon:([a-z-]+):([^}]+)\}\}/g, (_, name, className) => icon(name, className))
      .replace(/\{\{feature:([a-z-]+):([^:]+):([^}]+)\}\}/g, (_, name, title, text) => feature(name, title, text))
      .replace(/\{\{spec:([a-z-]+):([^:]+):([^}]+)\}\}/g, (_, name, title, text) => spec(name, title, text))
  },
}

export default defineConfig({
  plugins: [tailwindcss(), expandPlaceholders],
})
