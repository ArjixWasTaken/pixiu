<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Jobs
        <template #meta>
          <span>{{ running.length }} running · {{ waiting.length }} waiting</span>
        </template>
        <template #controls>
          <Btn v-if="done.length" size="small" variant="ghost" @click.prevent="clearFinished">Clear finished</Btn>
        </template>
      </ScreenHeader>
    </template>

    <ScreenEmptyState v-if="loaded && !jobs.length">
      <template #icon>
        <Icon :icon="faListCheck" />
      </template>
      Nothing going on
      <span class="secondary block"
        >Grab something from <a :href="url('hunt')">Hunt</a> and its download shows up here.</span
      >
    </ScreenEmptyState>

    <div v-else class="flex flex-col gap-8">
      <JobGroup v-if="failed.length" :jobs="failed" class="failed" title="Needs you" @retry="retry" />
      <JobGroup v-if="running.length" :jobs="running" title="Running" />
      <JobGroup v-if="waiting.length" :jobs="waiting" title="Waiting" />
      <JobGroup v-if="done.length" :jobs="done" title="Done" />
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faListCheck } from '@fortawesome/free-solid-svg-icons'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { HuntJob, JobState } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
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
