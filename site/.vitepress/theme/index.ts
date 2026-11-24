import '@fontsource-variable/inter'

import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'

import './custom.css'

import HomeScreenshot from './components/HomeScreenshot.vue'
import ReleaseCards from './components/ReleaseCards.vue'
import ReleaseMeta from './components/ReleaseMeta.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('HomeScreenshot', HomeScreenshot)
    app.component('ReleaseCards', ReleaseCards)
    app.component('ReleaseMeta', ReleaseMeta)
  },
} satisfies Theme
