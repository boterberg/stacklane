<script setup lang="ts">
import { useData, withBase } from 'vitepress'

import { data as releases } from '../releases.data'
import Icon from './Icon.vue'

const { frontmatter } = useData()
const latest = releases[0]
</script>

<template>
  <a v-if="frontmatter.layout === 'home' && latest" class="badge" :href="withBase(latest.url)">
    <span class="version">{{ latest.version }}</span>
    <span>{{ latest.title }}</span>
    <Icon name="arrow-right" :size="14" />
  </a>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin: 0 auto 28px;
  padding: 4px 12px 4px 4px;
  border: 1px solid var(--sl-border);
  border-radius: 999px;
  background: var(--sl-surface);
  color: var(--vp-c-text-2);
  font-size: 13px;
  font-weight: 500;
  text-decoration: none;
  transition: border-color 0.2s, color 0.2s;
}

.badge:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-text-1);
}

.version {
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
}
</style>
