<script setup lang="ts">
import { computed } from 'vue'

const sources = import.meta.glob('../../../public/icons/*.svg', { query: '?raw', import: 'default', eager: true }) as Record<string, string>

const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 20 })

const markup = computed(() => {
  const svg = sources[`../../../public/icons/${props.name}.svg`] ?? ''
  return svg.replace('width="24" height="24"', `width="${props.size}" height="${props.size}" aria-hidden="true"`)
})
</script>

<template>
  <span class="icon" v-html="markup" />
</template>

<style scoped>
.icon {
  display: inline-flex;
  flex-shrink: 0;
}
</style>
