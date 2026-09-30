<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Jobs
        <template #meta>
          <span>{{ running.length }} running · {{ waiting.length }} waiting</span>
        </template>
        <template #controls>
          <M3Button v-if="done.length" variant="text" @click.prevent="clearFinished">Clear finished</M3Button>
        </template>
      </ScreenHeader>
    </template>

    <ScreenEmptyState v-if="loaded && !jobs.length">
      <template #icon>
        <M3Icon :size="64" name="checklist" />
      </template>
      Nothing going on
      <span class="secondary block"
        >Download something from <a :href="url('hunt')">Discover</a> and it shows up here.</span
      >
    </ScreenEmptyState>

    <div v-else class="flex flex-col gap-6 pt-1" data-vue="JobsScreen">
      <JobGroup v-if="failed.length" :jobs="failed" class="failed" title="Needs you" @retry="retry" />
      <JobGroup v-if="running.length" :jobs="running" title="Running" />
      <JobGroup v-if="waiting.length" :jobs="waiting" title="Waiting" />
      <JobGroup v-if="done.length" :jobs="done" title="Done" />
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { HuntJob, JobState } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import JobGroup from '@/components/screens/hunting/JobGroup.vue'

const { url } = useRouter()
const { handleHttpError } = useErrorHandler('dialog')

const jobs = ref<HuntJob[]>([])
const loaded = ref(false)

const inState = (...states: JobState[]) => computed(() => jobs.value.filter(job => states.includes(job.state)))

const failed = inState('failed')
const running = inState('running')
// Oldest first, as they will run.
const queued = inState('queued', 'paused')
const waiting = computed(() => [...queued.value].reverse())
const done = inState('done')

const fetchJobs = async () => {
  try {
    jobs.value = await huntingService.jobs()
    loaded.value = true
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const retry = async (job: HuntJob) => {
  try {
    await huntingService.retryJob(job.id)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const clearFinished = async () => {
  try {
    await huntingService.clearFinishedJobs()
    await fetchJobs()
    await huntingStore.refresh()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchJobs)
eventBus.on('HUNT_JOBS_CHANGED', fetchJobs)
onBeforeUnmount(() => eventBus.off('HUNT_JOBS_CHANGED', fetchJobs))
</script>
