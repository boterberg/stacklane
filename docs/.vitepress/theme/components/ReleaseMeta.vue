<script setup lang="ts">
import { useData } from 'vitepress'
import { computed } from 'vue'

const { frontmatter } = useData()
const repository = 'https://github.com/boterberg/stacklane'

const date = computed(() => new Intl.DateTimeFormat('nl-NL', { dateStyle: 'long' }).format(new Date(frontmatter.value.date)))
const version = computed(() => frontmatter.value.version as string)
const deb = computed(() => `stacklane_${version.value.replace(/^v/, '')}_amd64.deb`)
</script>

<template>
  <div class="meta">
    <span>{{ date }}</span>
    <a :href="`${repository}/releases/tag/${version}`">GitHub-release</a>
    <a :href="`${repository}/releases/download/${version}/${deb}`">{{ deb }}</a>
  </div>
</template>

<style scoped>
.meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
  margin: -8px 0 24px;
  font-size: 14px;
  color: var(--vp-c-text-2);
}
</style>
