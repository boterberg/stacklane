import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { defineConfig } from 'vitepress'

const repository = 'https://github.com/boterberg/stacklane'
const site = 'https://stacklane.io'

function escapeXml(value: string) {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function frontmatterValue(source: string, key: string) {
  const match = source.match(new RegExp(`^${key}:\\s*(.+)$`, 'm'))
  return match ? match[1].trim().replace(/^['"]|['"]$/g, '') : ''
}

function writeReleaseFeed(srcDir: string, outDir: string) {
  const releases = readdirSync(join(srcDir, 'releases'))
    .filter((file) => /^v[\d.]+\.md$/.test(file))
    .map((file) => {
      const source = readFileSync(join(srcDir, 'releases', file), 'utf8')
      return {
        slug: file.replace(/\.md$/, ''),
        version: frontmatterValue(source, 'version'),
        title: frontmatterValue(source, 'title'),
        summary: frontmatterValue(source, 'summary'),
        date: frontmatterValue(source, 'date'),
      }
    })
    .sort((left, right) => right.date.localeCompare(left.date) || right.version.localeCompare(left.version, undefined, { numeric: true }))

  const items = releases
    .map(
      (release) => `    <item>
      <title>${escapeXml(`${release.version}: ${release.title}`)}</title>
      <link>${site}/releases/${release.slug}</link>
      <guid>${site}/releases/${release.slug}</guid>
      <pubDate>${new Date(release.date).toUTCString()}</pubDate>
      <description>${escapeXml(release.summary)}</description>
    </item>`,
    )
    .join('\n')

  writeFileSync(
    join(outDir, 'releases', 'feed.xml'),
    `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Stacklane releases</title>
    <link>${site}/releases/</link>
    <description>Nieuwe versies van Stacklane</description>
    <language>nl</language>
${items}
  </channel>
</rss>
`,
  )
}

export default defineConfig({
  lang: 'nl-NL',
  title: 'Stacklane',
  description: 'Stacklane is een klantpanel voor hostingbedrijven. Je installeert het met apt op je eigen server.',
  cleanUrls: true,
  lastUpdated: true,
  outDir: 'dist',
  buildEnd(config) {
    writeReleaseFeed(config.srcDir, config.outDir)
  },
  rewrites: {
    'getting-started/:page': 'docs/getting-started/:page',
    'installation/:page': 'docs/installation/:page',
    'guides/:page': 'docs/guides/:page',
    'contribute/:page': 'docs/contribute/:page',
  },
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico', sizes: 'any' }],
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
    ['link', { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' }],
    ['meta', { property: 'og:image', content: 'https://stacklane.io/panel-dashboard.webp' }],
    ['link', { rel: 'alternate', type: 'application/rss+xml', title: 'Stacklane releases', href: '/releases/feed.xml' }],
  ],
  themeConfig: {
    logo: { light: '/brand/stacklane-horizontal.svg', dark: '/brand/stacklane-horizontal-dark.svg', alt: 'Stacklane' },
    siteTitle: false,
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Docs', link: '/docs/getting-started/introduction', activeMatch: '/docs/' },
      { text: 'Releases', link: '/releases/', activeMatch: '/releases/' },
      { text: 'Demo', link: 'https://demo.stacklane.io/login' },
      {
        text: 'Over',
        items: [
          { text: 'Merk', link: '/brand' },
          { text: 'Broncode', link: repository },
          { text: 'Licentie', link: `${repository}/blob/main/LICENSE` },
        ],
      },
    ],
    socialLinks: [{ icon: 'github', link: repository }],
    sidebar: {
      '/docs/': [
        {
          text: 'Aan de slag',
          items: [{ text: 'Introductie', link: '/docs/getting-started/introduction' }],
        },
        {
          text: 'Installeren',
          items: [
            { text: 'Installeren', link: '/docs/installation/install' },
            { text: 'Webserver en HTTPS', link: '/docs/installation/webserver' },
            { text: 'Configuratie', link: '/docs/installation/configuration' },
            { text: 'Bijwerken', link: '/docs/installation/updating' },
          ],
        },
        {
          text: 'Handleidingen',
          items: [
            { text: 'Tickets', link: '/docs/guides/tickets' },
            { text: 'Gameservers', link: '/docs/guides/gameservers' },
            { text: 'VPS', link: '/docs/guides/vps' },
            { text: 'Facturatie', link: '/docs/guides/billing' },
            { text: 'Importeren', link: '/docs/guides/import' },
            { text: 'API', link: '/docs/guides/api' },
            { text: 'Veelgestelde vragen', link: '/docs/guides/faq' },
          ],
        },
        {
          text: 'Bijdragen',
          items: [{ text: 'Meebouwen aan Stacklane', link: '/docs/contribute/how-to-help' }],
        },
      ],
    },
    outline: { level: [2, 3], label: 'Op deze pagina' },
    lastUpdated: { text: 'Laatst bijgewerkt', formatOptions: { dateStyle: 'medium' } },
    docFooter: { prev: 'Vorige', next: 'Volgende' },
    darkModeSwitchLabel: 'Thema',
    lightModeSwitchTitle: 'Licht thema',
    darkModeSwitchTitle: 'Donker thema',
    sidebarMenuLabel: 'Menu',
    returnToTopLabel: 'Terug naar boven',
    langMenuLabel: 'Taal',
    notFound: { title: 'Pagina niet gevonden', quote: 'Deze pagina bestaat niet (meer).', linkText: 'Naar de homepage' },
    search: {
      provider: 'local',
      options: {
        translations: {
          button: { buttonText: 'Zoeken', buttonAriaLabel: 'Zoeken' },
          modal: {
            displayDetails: 'Details tonen',
            resetButtonTitle: 'Wissen',
            backButtonTitle: 'Sluiten',
            noResultsText: 'Geen resultaten voor',
            footer: { selectText: 'kiezen', navigateText: 'navigeren', closeText: 'sluiten' },
          },
        },
      },
    },
  },
})
