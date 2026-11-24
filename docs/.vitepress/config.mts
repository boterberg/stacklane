import { defineConfig } from 'vitepress'

const repository = 'https://github.com/boterberg/stacklane'

export default defineConfig({
  lang: 'nl-NL',
  title: 'Stacklane',
  description: 'Open-source klantpanel voor hostingbedrijven: gameservers, VPS, domeinen, webhosting, support en facturatie.',
  cleanUrls: true,
  lastUpdated: true,
  outDir: 'dist',
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
  ],
  themeConfig: {
    logo: { light: '/brand/stacklane-horizontal.svg', dark: '/brand/stacklane-horizontal-dark.svg', alt: 'Stacklane' },
    siteTitle: false,
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Docs', link: '/docs/getting-started/introduction', activeMatch: '/docs/' },
      { text: 'Releases', link: '/releases/', activeMatch: '/releases/' },
      {
        text: 'Over',
        items: [
          { text: 'Merk', link: '/brand' },
          { text: 'Demo', link: 'https://demo.stacklane.io' },
          { text: 'Broncode', link: repository },
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
            { text: 'Installatie', link: '/docs/installation/install' },
            { text: 'Webserver en HTTPS', link: '/docs/installation/webserver' },
            { text: 'Configuratie', link: '/docs/installation/configuration' },
            { text: 'Updaten', link: '/docs/installation/updating' },
          ],
        },
        {
          text: 'Handleidingen',
          items: [
            { text: 'Tickets', link: '/docs/guides/tickets' },
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
    editLink: { pattern: `${repository}/edit/main/docs/:path`, text: 'Deze pagina bewerken op GitHub' },
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
    footer: {
      copyright: '© 2026 Stacklane',
    },
  },
})
