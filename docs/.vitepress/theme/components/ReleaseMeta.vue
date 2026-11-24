<script setup lang="ts">
import { useData } from 'vitepress'
import { computed } from 'vue'

import Icon from './Icon.vue'

const { frontmatter } = useData()

const date = computed(() => new Intl.DateTimeFormat('nl-NL', { dateStyle: 'long' }).format(new Date(frontmatter.value.date)))
</script>

<template>
  <div class="meta">
    <span>{{ date }}</span>
    <span class="command">
      <Icon name="terminal" :size="14" />
      <code>sudo stacklane update</code>
    </span>
  </div>
</template>

<style scoped>
.meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 20px;
  margin: -8px 0 24px;
  color: var(--vp-c-text-2);
  font-size: 14px;
}

.command {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
