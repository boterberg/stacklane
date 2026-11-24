<script setup lang="ts">
import { withBase } from 'vitepress'

import { data as releases } from '../releases.data'

const gradients = [
  'linear-gradient(135deg, #c7d2fe, #a5f3fc)',
  'linear-gradient(135deg, #ddd6fe, #fbcfe8)',
  'linear-gradient(135deg, #bbf7d0, #bfdbfe)',
  'linear-gradient(135deg, #fde68a, #fecaca)',
]

function formatDate(value: string) {
  return new Intl.DateTimeFormat('nl-NL', { dateStyle: 'long' }).format(new Date(value))
}
</script>

<template>
  <div class="grid">
    <a v-for="(release, index) in releases" :key="release.version" :href="withBase(release.url)" class="card">
      <div class="banner" :style="{ background: gradients[index % gradients.length] }">
        <span>{{ release.version }}</span>
      </div>
      <div class="body">
        <p class="title">{{ release.title }}</p>
        <p class="summary">{{ release.summary }}</p>
        <p class="footer">
          <time :datetime="release.date">{{ formatDate(release.date) }}</time>
          <span class="read">Lees de notities →</span>
        </p>
      </div>
    </a>
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  gap: 24px;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  margin-top: 32px;
}

.card {
  display: block;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  background: var(--vp-c-bg-soft);
  color: inherit;
  text-decoration: none !important;
  transition: border-color 0.2s;
}

.card:hover {
  border-color: var(--vp-c-brand-1);
}

.banner {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 112px;
}

.banner span {
  color: #18181b;
  font-size: 26px;
  font-weight: 500;
  letter-spacing: -0.02em;
}

.body {
  padding: 16px 20px 20px;
}

.title {
  margin: 0;
  font-weight: 600;
  color: var(--vp-c-text-1);
}

.summary {
  margin: 8px 0 16px;
  font-size: 14px;
  line-height: 22px;
  color: var(--vp-c-text-2);
}

.footer {
  display: flex;
  justify-content: space-between;
  margin: 0;
  font-size: 13px;
  color: var(--vp-c-text-2);
}

.read {
  font-weight: 500;
  color: var(--vp-c-text-1);
}
</style>
