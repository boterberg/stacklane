import '@fontsource-variable/inter'

import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import { h } from 'vue'

import './custom.css'

import HeroBadge from './components/HeroBadge.vue'
import HomeSections from './components/HomeSections.vue'
import ReleaseMeta from './components/ReleaseMeta.vue'
import ReleasesPage from './components/ReleasesPage.vue'
import SiteFooter from './components/SiteFooter.vue'

export default {
  extends: DefaultTheme,
  Layout: () =>
    h(DefaultTheme.Layout, null, {
      'home-hero-info-before': () => h(HeroBadge),
      'layout-bottom': () => h(SiteFooter),
    }),
  enhanceApp({ app }) {
    app.component('HomeSections', HomeSections)
    app.component('ReleasesPage', ReleasesPage)
    app.component('ReleaseMeta', ReleaseMeta)
  },
} satisfies Theme
