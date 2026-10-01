<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Jobs
        <template #meta>
          <span
            >{{ running.length }} running · {{ waiting.length }} waiting<template v-if="failed.length">
              · {{ failed.length }} failed</template
            ></span
          >
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
import { useQuery } from '@tanstack/vue-query'
import { computed, watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { HuntJob, JobState } from '@/services/huntingService'
import { queryClient } from '@/services/queryClient'
import { useHuntingStore } from '@/stores/huntingStore'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import JobGroup from '@/components/screens/hunting/JobGroup.vue'

const huntingStore = useHuntingStore()

const { url } = useRouter()
const { handleHttpError } = useErrorHandler('dialog')

// Fetched again whenever the server says the job board changed (see huntingStore).
const {
  data,
  isSuccess: loaded,
  error,
} = useQuery({ queryKey: ['hunting', 'jobs'], queryFn: () => huntingService.jobs() })
watch(error, error => error && handleHttpError(error))

const jobs = computed(() => data.value ?? [])

const inState = (...states: JobState[]) => computed(() => jobs.value.filter(job => states.includes(job.state)))

const failed = inState('failed')
const running = inState('running')
// Oldest first, as they will run.
const queued = inState('queued', 'paused')
const waiting = computed(() => [...queued.value].reverse())
const done = inState('done')

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
    await queryClient.invalidateQueries({ queryKey: ['hunting', 'jobs'] })
    await huntingStore.refresh()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}
</script>
