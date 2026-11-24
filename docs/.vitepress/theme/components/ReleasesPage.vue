<script setup lang="ts">
import { withBase } from 'vitepress'
import { computed, onMounted, reactive, ref } from 'vue'

import { data, type Release, type ReleaseChanges } from '../releases.data'
import CopyCommand from './CopyCommand.vue'
import Icon from './Icon.vue'

type Language = 'nl' | 'en'

const copy = {
  nl: {
    heading: 'Wat er nieuw is in Stacklane',
    lead: 'Elke versie komt in de pakketbron op stacklane.io terecht. Je server haalt hem op met één commando, en vooraf maakt Stacklane een back-up van je database.',
    feed: 'Volg nieuwe versies via RSS',
    latest: 'Nieuwste versie',
    read: 'Lees wat er in {version} zit',
    versions: 'Versies',
    newest: 'Nieuwste',
    building: 'In opbouw',
    buildingNote: 'Deze versie is in opbouw. Het pakket staat nog niet in de pakketbron; zodra het er is, installeer je het met apt.',
    all: 'Alle wijzigingen',
    notes: 'Volledige notities',
    kinds: { nieuw: 'Nieuw', verbeterd: 'Verbeterd', opgelost: 'Opgelost' },
    locale: 'nl-NL',
  },
  en: {
    heading: "What's new in Stacklane",
    lead: 'Every version lands in the package repository on stacklane.io. Your server fetches it with one command, and Stacklane backs up your database first.',
    feed: 'Follow new versions via RSS',
    latest: 'Latest version',
    read: "Read what's in {version}",
    versions: 'Versions',
    newest: 'Latest',
    building: 'In progress',
    buildingNote: 'This version is still being built. The package is not in the repository yet; once it is, you install it with apt.',
    all: 'All changes',
    notes: 'Full notes',
    kinds: { nieuw: 'New', verbeterd: 'Improved', opgelost: 'Fixed' },
    locale: 'en-GB',
  },
}

const language = ref<Language>('nl')
const text = computed(() => copy[language.value])
const activeImage = reactive<Record<string, number>>({})

function localized(release: Release): Release {
  return language.value === 'en' ? { ...release, ...release.en } : release
}

const releases = computed(() => data.map(localized))
const latest = computed(() => releases.value[0])

const kinds: Array<keyof ReleaseChanges> = ['nieuw', 'verbeterd', 'opgelost']

onMounted(() => {
  try {
    const wanted = new URLSearchParams(window.location.search).get('lang') ?? window.localStorage.getItem('stacklane-releases-lang')
    if (wanted === 'en' || wanted === 'nl') language.value = wanted
  } catch {
    language.value = 'nl'
  }
})

function choose(next: Language) {
  language.value = next
  try {
    window.localStorage.setItem('stacklane-releases-lang', next)
  } catch {
    return
  }
}

function notesUrl(release: Release) {
  return withBase(language.value === 'en' ? release.url.replace('/releases/', '/releases/en/') : release.url)
}

function formatDate(value: string) {
  return new Intl.DateTimeFormat(text.value.locale, { day: 'numeric', month: 'long', year: 'numeric' }).format(new Date(value))
}

function anchor(version: string) {
  return version.replaceAll('.', '-')
}

function changeCount(changes: ReleaseChanges) {
  return kinds.reduce((total, kind) => total + (changes[kind]?.length ?? 0), 0)
}
</script>

<template>
  <div class="releases">
    <header class="top">
      <div class="intro">
        <div class="eyebrow-row">
          <p class="eyebrow">Releases</p>
          <div class="languages" role="group" aria-label="Taal / Language">
            <button type="button" :class="{ active: language === 'nl' }" :aria-pressed="language === 'nl'" @click="choose('nl')">NL</button>
            <button type="button" :class="{ active: language === 'en' }" :aria-pressed="language === 'en'" @click="choose('en')">EN</button>
          </div>
        </div>
        <h1>{{ text.heading }}</h1>
        <p class="lead">{{ text.lead }}</p>
        <a class="feed" :href="withBase('/releases/feed.xml')">
          <Icon name="rss" :size="15" />
          {{ text.feed }}
        </a>
      </div>

      <aside v-if="latest" class="latest">
        <p class="latest-label">{{ text.latest }}</p>
        <p class="latest-version">{{ latest.version }}</p>
        <p class="latest-date">
          <Icon name="calendar" :size="14" />
          {{ formatDate(latest.date) }}
        </p>
        <p v-if="latest.building" class="building-note">{{ text.buildingNote }}</p>
        <CopyCommand v-else command="sudo stacklane update" />
        <a class="latest-link" :href="notesUrl(latest)">
          {{ text.read.replace('{version}', latest.version) }}
          <Icon name="arrow-right" :size="14" />
        </a>
      </aside>
    </header>

    <div class="layout">
      <nav class="rail" :aria-label="text.versions">
        <p class="rail-title">{{ text.versions }}</p>
        <a v-for="release in releases" :key="release.version" :href="`#${anchor(release.version)}`" class="rail-item">
          <span class="rail-version">{{ release.version }}</span>
          <span class="rail-date">{{ formatDate(release.date) }}</span>
        </a>
      </nav>

      <div class="list">
        <article v-for="(release, index) in releases" :id="anchor(release.version)" :key="release.version" class="release">
          <div class="release-head">
            <span class="chip chip-version">{{ release.version }}</span>
            <span v-if="release.building" class="chip chip-building">{{ text.building }}</span>
            <span v-else-if="index === 0" class="chip chip-latest">{{ text.newest }}</span>
            <span v-if="release.label" class="chip">{{ release.label }}</span>
            <time :datetime="release.date" class="release-date">{{ formatDate(release.date) }}</time>
          </div>

          <h2>
            <a :href="notesUrl(release)">{{ release.title }}</a>
          </h2>
          <p class="summary">{{ release.summary }}</p>
          <p v-if="release.building" class="building-note">{{ text.buildingNote }}</p>

          <figure v-if="release.images.length" class="shot">
            <div class="window">
              <div class="window-bar">
                <span /><span /><span />
                <div v-if="release.images.length > 1" class="tabs" role="tablist">
                  <button
                    v-for="(image, imageIndex) in release.images"
                    :key="image.src"
                    type="button"
                    role="tab"
                    :aria-selected="(activeImage[release.version] ?? 0) === imageIndex"
                    :class="{ active: (activeImage[release.version] ?? 0) === imageIndex }"
                    @click="activeImage[release.version] = imageIndex"
                  >
                    {{ image.tab ?? image.alt }}
                  </button>
                </div>
              </div>
              <img
                v-for="(image, imageIndex) in release.images"
                v-show="(activeImage[release.version] ?? 0) === imageIndex"
                :key="image.src"
                :src="withBase(image.src)"
                :alt="image.alt"
                loading="lazy"
              />
            </div>
            <figcaption>{{ release.images[activeImage[release.version] ?? 0].caption }}</figcaption>
          </figure>

          <div v-if="release.highlights.length" class="highlights">
            <div v-for="highlight in release.highlights" :key="highlight.title" class="highlight">
              <span class="highlight-icon"><Icon :name="highlight.icon" :size="18" /></span>
              <h3>{{ highlight.title }}</h3>
              <p>{{ highlight.text }}</p>
            </div>
          </div>

          <div v-if="changeCount(release.changes)" class="changes">
            <h3 class="changes-title">{{ text.all }}</h3>
            <ul>
              <template v-for="kind in kinds" :key="kind">
                <li v-for="item in release.changes[kind] ?? []" :key="item">
                  <span class="tag" :class="`tag-${kind}`">{{ text.kinds[kind] }}</span>
                  <span>{{ item }}</span>
                </li>
              </template>
            </ul>
          </div>

          <div class="release-foot">
            <CopyCommand v-if="!release.building" command="sudo stacklane update" class="foot-command" />
            <a :href="notesUrl(release)" class="foot-link">
              {{ text.notes }}
              <Icon name="arrow-right" :size="14" />
            </a>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<style scoped>
.releases {
  margin: 0 auto;
  max-width: 1152px;
  padding: 48px 24px 96px;
}

.top {
  display: grid;
  gap: 32px;
  align-items: end;
  padding-bottom: 48px;
  border-bottom: 1px solid var(--vp-c-divider);
}

@media (min-width: 960px) {
  .top {
    grid-template-columns: 1fr 360px;
  }
}

.eyebrow {
  margin: 0 0 12px;
  color: var(--vp-c-brand-1);
  font-size: 14px;
  font-weight: 600;
}

h1 {
  margin: 0;
  color: var(--vp-c-text-1);
  font-size: 40px;
  font-weight: 700;
  letter-spacing: -0.035em;
  line-height: 1.1;
}

.lead {
  margin: 16px 0 0;
  max-width: 560px;
  color: var(--vp-c-text-2);
  font-size: 17px;
  line-height: 1.6;
}

.feed {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-top: 20px;
  color: var(--vp-c-text-2);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
}

.feed:hover {
  color: var(--vp-c-brand-1);
}

.latest {
  padding: 24px;
  border: 1px solid var(--sl-border);
  border-radius: 16px;
  background: var(--sl-surface);
}

.latest-label {
  margin: 0;
  color: var(--vp-c-text-2);
  font-size: 13px;
  font-weight: 500;
}

.latest-version {
  margin: 4px 0 0;
  color: var(--vp-c-text-1);
  font-size: 34px;
  font-weight: 700;
  letter-spacing: -0.03em;
}

.latest-date {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 4px 0 20px;
  color: var(--vp-c-text-2);
  font-size: 14px;
}

.latest-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 16px;
  color: var(--vp-c-brand-1);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
}

.layout {
  display: grid;
  gap: 40px;
  margin-top: 48px;
}

@media (min-width: 1024px) {
  .layout {
    grid-template-columns: 180px 1fr;
  }
}

.rail {
  display: none;
}

@media (min-width: 1024px) {
  .rail {
    position: sticky;
    top: calc(var(--vp-nav-height) + 32px);
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-self: start;
  }
}

.rail-title {
  margin: 0 0 8px;
  color: var(--vp-c-text-3);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.rail-item {
  display: flex;
  flex-direction: column;
  padding: 8px 10px;
  border-radius: 8px;
  text-decoration: none;
}

.rail-item:hover {
  background: var(--sl-surface);
}

.rail-version {
  color: var(--vp-c-text-1);
  font-size: 14px;
  font-weight: 600;
}

.rail-date {
  color: var(--vp-c-text-2);
  font-size: 12px;
}

.list {
  display: flex;
  flex-direction: column;
  gap: 40px;
}

.release {
  scroll-margin-top: calc(var(--vp-nav-height) + 24px);
  padding: 32px;
  border: 1px solid var(--sl-border);
  border-radius: 20px;
}

.release-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.chip {
  padding: 3px 10px;
  border: 1px solid var(--sl-border);
  border-radius: 999px;
  color: var(--vp-c-text-2);
  font-size: 12px;
  font-weight: 500;
}

.chip-version {
  border-color: transparent;
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
  font-weight: 600;
}

.chip-latest {
  border-color: transparent;
  background: rgba(34, 197, 94, 0.14);
  color: #15803d;
}

:global(.dark) .chip-latest {
  color: #86efac;
}

.chip-building {
  border-color: transparent;
  background: rgba(245, 158, 11, 0.16);
  color: #b45309;
}

:global(.dark) .chip-building {
  color: #fcd34d;
}

.building-note {
  margin: 12px 0 16px;
  padding: 12px 14px;
  border: 1px solid rgba(245, 158, 11, 0.35);
  border-radius: 10px;
  background: rgba(245, 158, 11, 0.08);
  color: var(--vp-c-text-1);
  font-size: 14px;
  line-height: 1.5;
}

.eyebrow-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.languages {
  display: inline-flex;
  padding: 3px;
  border-radius: 8px;
  background: var(--vp-c-default-soft);
}

.languages button {
  padding: 2px 10px;
  border-radius: 6px;
  color: var(--vp-c-text-2);
  font-size: 12px;
  font-weight: 600;
}

.languages button.active {
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
}

.release-date {
  margin-left: auto;
  color: var(--vp-c-text-2);
  font-size: 13px;
}

.release h2 {
  margin: 20px 0 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: -0.03em;
  line-height: 1.2;
}

.release h2 a {
  color: var(--vp-c-text-1);
  text-decoration: none;
}

.summary {
  margin: 12px 0 0;
  max-width: 680px;
  color: var(--vp-c-text-2);
  font-size: 16px;
  line-height: 1.6;
}

.shot {
  margin: 28px 0 0;
}

.tabs {
  display: flex;
  gap: 4px;
  margin-left: auto;
}

.tabs button {
  padding: 3px 10px;
  border-radius: 6px;
  color: #a1a1aa;
  font-size: 12px;
  font-weight: 500;
}

.tabs button:hover {
  color: #fafafa;
}

.tabs button.active {
  background: rgba(255, 255, 255, 0.1);
  color: #fafafa;
}

.window {
  overflow: hidden;
  border: 1px solid var(--sl-border);
  border-radius: 12px;
  background: #18181b;
}

.window-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 9px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.window-bar span {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #3f3f46;
}

.window img {
  display: block;
  width: 100%;
  height: auto;
}

figcaption {
  margin-top: 10px;
  color: var(--vp-c-text-2);
  font-size: 13px;
}

.highlights {
  display: grid;
  gap: 16px;
  margin-top: 32px;
}

@media (min-width: 768px) {
  .highlights {
    grid-template-columns: repeat(3, 1fr);
  }
}

.highlight {
  padding: 20px;
  border-radius: 14px;
  background: var(--sl-surface);
}

.highlight-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
}

.highlight h3 {
  margin: 14px 0 0;
  color: var(--vp-c-text-1);
  font-size: 15px;
  font-weight: 600;
}

.highlight p {
  margin: 6px 0 0;
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 1.55;
}

.changes {
  margin-top: 32px;
}

.changes-title {
  margin: 0 0 12px;
  color: var(--vp-c-text-1);
  font-size: 15px;
  font-weight: 600;
}

.changes ul {
  margin: 0;
  padding: 0;
  list-style: none;
  border-top: 1px solid var(--vp-c-divider);
}

.changes li {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--vp-c-divider);
  color: var(--vp-c-text-1);
  font-size: 14px;
  line-height: 1.55;
}

.tag {
  flex-shrink: 0;
  width: 78px;
  padding: 1px 0;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  text-align: center;
}

.tag-nieuw {
  background: rgba(34, 197, 94, 0.14);
  color: #15803d;
}

.tag-verbeterd {
  background: rgba(59, 130, 246, 0.14);
  color: #1d4ed8;
}

.tag-opgelost {
  background: rgba(245, 158, 11, 0.16);
  color: #b45309;
}

:global(.dark) .tag-nieuw {
  color: #86efac;
}

:global(.dark) .tag-verbeterd {
  color: #93c5fd;
}

:global(.dark) .tag-opgelost {
  color: #fcd34d;
}

.release-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 32px;
}

.foot-command {
  min-width: 280px;
}

.foot-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--vp-c-brand-1);
  font-size: 14px;
  font-weight: 500;
  text-decoration: none;
}

@media (max-width: 640px) {
  .release {
    padding: 20px;
  }

  h1 {
    font-size: 32px;
  }

  .release h2 {
    font-size: 24px;
  }

  .release-date {
    margin-left: 0;
    width: 100%;
  }
}
</style>
