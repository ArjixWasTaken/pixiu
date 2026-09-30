<template>
  <M3Chip v-if="standing === 'hoarded'" class="self-start" icon="check">In your library</M3Chip>
  <span v-else-if="standing === 'pending'" class="pending m3-label-large">
    <M3ProgressIndicator :size="20" variant="circular" />
    On its way
  </span>
  <M3Button v-else class="self-start" icon="download" variant="tonal" @click.prevent="emit('grab')">Download</M3Button>
</template>

<script lang="ts" setup>
import type { Standing } from '@/services/huntingService'

import M3Button from '@/components/m3/M3Button.vue'
import M3Chip from '@/components/m3/M3Chip.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

defineProps<{ standing: Standing }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>

<style scoped>
.pending {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  color: var(--schemes-on-surface-variant);
}
</style>
