import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const root = new URL('../src/', import.meta.url).pathname
const dictionary = readFileSync(join(root, 'locales/en.ts'), 'utf8')
const known = new Set([...dictionary.matchAll(/^ {2}("(?:[^"\\]|\\.)*"): /gm)].map((match) => JSON.parse(match[1])))

function files(directory) {
  return readdirSync(directory).flatMap((name) => {
    const path = join(directory, name)
    if (statSync(path).isDirectory()) return name === 'locales' ? [] : files(path)
    return /\.tsx?$/.test(name) ? [path] : []
  })
}

const labelFiles = ['app/navigation.ts', 'app/router.tsx', 'lib/labels.ts', 'components/admin/UpdatePanel.tsx', 'pages/admin/InvoicePage.tsx', 'pages/account/SecurityPage.tsx', 'pages/account/ApiTokensTab.tsx']
const missing = new Map()

for (const file of files(root)) {
  const source = readFileSync(file, 'utf8')
  const relative = file.slice(root.length)
  const texts = [...source.matchAll(/\bt\(\s*'((?:[^'\\]|\\.)*)'/g)].map((match) => match[1].replace(/\\'/g, "'"))
  if (labelFiles.includes(relative)) {
    texts.push(...[...source.matchAll(/(?:^\s*\w+: |label: |\[\s*'[^']*',\s*)'([^']*[a-zA-Z][^']*)'/gm)].map((match) => match[1]).filter((text) => !/^[a-z_/]+$/.test(text) && !text.startsWith('/') && !text.startsWith('@')))
  }
  for (const text of texts) {
    if (!known.has(text)) missing.set(text, relative)
  }
}

if (missing.size > 0) {
  for (const [text, file] of missing) console.error(`${file}: no English text for "${text}"`)
  process.exit(1)
}
console.log(`i18n: ${known.size} texts, all translated`)
