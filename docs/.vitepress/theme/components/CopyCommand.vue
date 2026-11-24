<script setup lang="ts">
import { ref } from 'vue'

import Icon from './Icon.vue'

const props = defineProps<{ command: string }>()
const copied = ref(false)

async function copy() {
  try {
    await navigator.clipboard.writeText(props.command)
    copied.value = true
    setTimeout(() => (copied.value = false), 1600)
  } catch {
    copied.value = false
  }
}
</script>

<template>
  <div class="command">
    <span class="prompt">$</span>
    <code>{{ command }}</code>
    <button type="button" :aria-label="copied ? 'Gekopieerd' : 'Kopiëren'" :title="copied ? 'Gekopieerd' : 'Kopiëren'" @click="copy">
      <Icon :name="copied ? 'check' : 'copy'" :size="15" />
    </button>
  </div>
</template>

<style scoped>
.command {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 8px 8px 14px;
  border-radius: 10px;
  background: #18181b;
  color: #e4e4e7;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
}

.prompt {
  color: #71717a;
}

code {
  flex: 1;
  overflow-x: auto;
  white-space: nowrap;
}

button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 7px;
  color: #a1a1aa;
  transition: background 0.15s, color 0.15s;
}

button:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #fafafa;
}
</style>
