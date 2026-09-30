<template>
  <section data-vue="JobGroup">
    <h2 class="title m3-title-small mb-2">{{ title }}</h2>
    <ul class="flex flex-col gap-2">
      <JobRow v-for="job in jobs" :key="job.id" :job @retry="emit('retry', job)" />
    </ul>
  </section>
</template>

<script lang="ts" setup>
import type { HuntJob } from '@/services/huntingService'

import JobRow from '@/components/screens/hunting/JobRow.vue'

defineProps<{ title: string; jobs: HuntJob[] }>()
const emit = defineEmits<{ (e: 'retry', job: HuntJob): void }>()
</script>

<style scoped>
.title {
  color: var(--schemes-on-surface-variant);

  .failed & {
    color: var(--schemes-error);
  }
}
</style>
