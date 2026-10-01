<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Orphans
        <template #meta>
          <span>{{ pluralize(orphans, 'orphan') }} · {{ formatBytes(totalSize) }}</span>
        </template>
        <template v-if="orphans.length" #controls>
          <div class="flex gap-2">
            <M3Button :disabled="!selected.length" variant="tonal" @click.prevent="keep">Keep</M3Button>
            <M3Button :disabled="!selected.length" class="danger" variant="outlined" @click.prevent="deleteSelected">
              Delete
            </M3Button>
            <M3Button class="danger" variant="text" @click.prevent="deleteAll">Delete all</M3Button>
          </div>
        </template>
      </ScreenHeader>
    </template>

    <p v-if="orphans.length" class="m3-body-large text-(--schemes-on-surface-variant) pt-1 mb-3 max-w-[64ch]">
      Nothing keeps these any more: they left a watched playlist, were excluded, or their watch was removed. píxiū never
      deletes music on its own. Keep them for good, or delete them from disk.
    </p>

    <ScreenEmptyState v-if="loaded && !orphans.length">
      <template #icon>
        <M3Icon :size="64" name="verified" />
      </template>
      Everything in your library is wanted
      <span class="secondary block">Songs show up here when nothing keeps them any more.</span>
    </ScreenEmptyState>

    <ul v-else data-vue="OrphansScreen">
      <li v-if="orphans.length" class="flex items-center gap-3 py-2 list-none">
        <M3Checkbox v-model="allSelected" aria-label="Select all" name="select_all" />
        <span class="m3-label-large">Select all</span>
      </li>
      <OrphanRow v-for="orphan in orphans" :key="orphan.song.id" v-model="selectedIds[orphan.song.id]" :orphan />
    </ul>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { computed, reactive, watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import { useHuntingStore } from '@/stores/huntingStore'
import { formatBytes, pluralize } from '@/utils/formatters'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3Checkbox from '@/components/m3/M3Checkbox.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import OrphanRow from '@/components/screens/hunting/OrphanRow.vue'

const huntingStore = useHuntingStore()

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const {
  data,
  isSuccess: loaded,
  error,
  refetch,
} = useQuery({
  queryKey: ['hunting', 'orphans'],
  queryFn: () => huntingService.orphans(),
})
watch(error, error => error && handleHttpError(error))

const orphans = computed(() => data.value?.orphans ?? [])
const totalSize = computed(() => data.value?.totalSize ?? 0)
const selectedIds = reactive<Record<string, boolean>>({})

// A new list: nothing in it is chosen yet.
watch(data, () => Object.keys(selectedIds).forEach(id => delete selectedIds[id]))

const selected = computed(() => orphans.value.filter(({ song }) => selectedIds[song.id]).map(({ song }) => song))

const allSelected = computed({
  get: () => orphans.value.length > 0 && selected.value.length === orphans.value.length,
  set: value => orphans.value.forEach(({ song }) => (selectedIds[song.id] = value)),
})

const after = async (message: string) => {
  toastSuccess(message)
  await refetch()
  await huntingStore.refresh()
}

const keep = async () => {
  try {
    const { kept } = await huntingService.keepOrphans(selected.value)
    await after(`Kept ${pluralize(kept, 'song')} for good.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const remove = async (songs: Song[] | 'all', count: number) => {
  if (!(await showConfirmDialog(`Delete ${pluralize(count, 'song')} from disk? This cannot be undone.`))) {
    return
  }

  try {
    const { deleted } = await huntingService.deleteOrphans(songs)
    await after(`Deleted ${pluralize(deleted, 'song')} from disk.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const deleteSelected = () => remove(selected.value, selected.value.length)
const deleteAll = () => remove('all', orphans.value.length)
</script>

<style scoped>
.danger {
  color: var(--schemes-error);
}
</style>
