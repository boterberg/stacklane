<script setup lang="ts">
import { useData, withBase } from 'vitepress'

const { frontmatter, page } = useData()

const columns = [
  {
    title: 'Product',
    links: [
      { text: 'Demo', href: 'https://demo.stacklane.io/login' },
      { text: 'Releases', href: '/releases/' },
      { text: 'Roadmap', href: '/#roadmap' },
    ],
  },
  {
    title: 'Documentatie',
    links: [
      { text: 'Introductie', href: '/docs/getting-started/introduction' },
      { text: 'Installeren', href: '/docs/installation/install' },
      { text: 'Bijwerken', href: '/docs/installation/updating' },
      { text: 'Tickets', href: '/docs/guides/tickets' },
    ],
  },
  {
    title: 'Over',
    links: [
      { text: 'Merk', href: '/brand' },
      { text: 'Broncode', href: 'https://github.com/boterberg/stacklane' },
      { text: 'MIT-licentie', href: 'https://github.com/boterberg/stacklane/blob/v1-stable/LICENSE' },
    ],
  },
]

function href(link: string) {
  return link.startsWith('http') ? link : withBase(link)
}
</script>

<template>
  <footer v-if="frontmatter.layout === 'home' || page.relativePath.startsWith('releases') || page.relativePath === 'brand.md'" class="footer">
    <div class="inner">
      <div class="brand">
        <img class="logo light" src="/brand/stacklane-horizontal.svg" alt="Stacklane" />
        <img class="logo dark" src="/brand/stacklane-horizontal-dark.svg" alt="Stacklane" />
        <p>Klantpanel voor hostingbedrijven, uitgebracht onder de MIT-licentie.</p>
      </div>
      <nav v-for="column in columns" :key="column.title" class="column">
        <p class="title">{{ column.title }}</p>
        <a v-for="link in column.links" :key="link.text" :href="href(link.href)">{{ link.text }}</a>
      </nav>
    </div>
    <p class="copyright">© 2026 Stacklane</p>
  </footer>
</template>

<style scoped>
.footer {
  border-top: 1px solid var(--vp-c-divider);
  padding: 48px 24px 32px;
}

.inner {
  display: grid;
  gap: 32px;
  margin: 0 auto;
  max-width: 1152px;
  grid-template-columns: 1fr;
}

@media (min-width: 768px) {
  .inner {
    grid-template-columns: 2fr 1fr 1fr 1fr;
  }
}

.logo {
  height: 22px;
}

.logo.dark,
:global(.dark) .logo.light {
  display: none;
}

:global(.dark) .logo.dark {
  display: block;
}

.brand p {
  margin: 12px 0 0;
  max-width: 280px;
  color: var(--vp-c-text-2);
  font-size: 14px;
}

.column {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.title {
  margin: 0 0 4px;
  color: var(--vp-c-text-1);
  font-size: 14px;
  font-weight: 600;
}

.column a {
  color: var(--vp-c-text-2);
  font-size: 14px;
  text-decoration: none;
}

.column a:hover {
  color: var(--vp-c-text-1);
}

.copyright {
  margin: 40px auto 0;
  max-width: 1152px;
  color: var(--vp-c-text-3);
  font-size: 13px;
}
</style>
