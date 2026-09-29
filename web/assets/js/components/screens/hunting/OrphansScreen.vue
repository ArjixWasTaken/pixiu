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
            <Btn :disabled="!selected.length" size="small" variant="success" @click.prevent="keep">Keep</Btn>
            <Btn :disabled="!selected.length" size="small" variant="destructive" @click.prevent="deleteSelected">
              Delete
            </Btn>
            <Btn size="small" variant="ghost" @click.prevent="deleteAll">Delete all</Btn>
          </div>
        </template>
      </ScreenHeader>
    </template>

    <p class="text-k-fg-70 mb-6 max-w-[64ch]">
      Nothing keeps these any more: they left a watched playlist, were excluded, or their watch was removed. píxiū never
      gives treasure back on its own. Keep them for good, or delete them from disk.
    </p>

    <ScreenEmptyState v-if="loaded && !orphans.length">
      <template #icon>
        <Icon :icon="faHeartCircleCheck" />
      </template>
      Everything in the hoard is wanted
      <span class="secondary block">Songs show up here when nothing keeps them any more.</span>
    </ScreenEmptyState>

    <ul v-else class="divide-y divide-k-fg-5">
      <li v-if="orphans.length" class="flex items-center gap-4 py-2 text-sm text-k-fg-70">
        <CheckBox v-model="allSelected" name="select_all" />
        Select all
      </li>
      <OrphanRow v-for="orphan in orphans" :key="orphan.song.id" v-model="selectedIds[orphan.song.id]" :orphan />
    </ul>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faHeartCircleCheck } from '@fortawesome/free-solid-svg-icons'
import { computed, onMounted, reactive, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Orphan } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { formatBytes, pluralize } from '@/utils/formatters'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import CheckBox from '@/components/ui/form/CheckBox.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import OrphanRow from '@/components/screens/hunting/OrphanRow.vue'

const { showConfirmDialog } = useDialogBox()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const orphans = ref<Orphan[]>([])
const totalSize = ref(0)
const loaded = ref(false)
const selectedIds = reactive<Record<string, boolean>>({})

const selected = computed(() => orphans.value.filter(({ song }) => selectedIds[song.id]).map(({ song }) => song))

const allSelected = computed({
  get: () => orphans.value.length > 0 && selected.value.length === orphans.value.length,
  set: value => orphans.value.forEach(({ song }) => (selectedIds[song.id] = value)),
})

const fetchOrphans = async () => {
  try {
    const result = await huntingService.orphans()
    orphans.value = result.orphans
    totalSize.value = result.totalSize
    loaded.value = true
    Object.keys(selectedIds).forEach(id => delete selectedIds[id])
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const after = async (message: string) => {
  toastSuccess(message)
  await fetchOrphans()
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

onMounted(fetchOrphans)
</script>
