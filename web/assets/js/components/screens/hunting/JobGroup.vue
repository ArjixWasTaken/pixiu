<template>
  <section>
    <h2 class="title text-sm uppercase tracking-widest text-k-fg-70 mb-3">{{ title }}</h2>
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

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.failed .title {
  @apply text-k-danger;
}
</style>
