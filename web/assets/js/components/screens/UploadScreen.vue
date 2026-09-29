<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Offerings
        <template #meta>
          <span>{{ pluralize(batches, 'batch') }} to review</span>
        </template>
      </ScreenHeader>
    </template>

    <div
      :class="{ droppable }"
      class="flex flex-col gap-8"
      @dragenter.prevent="onDragEnter"
      @dragleave.prevent="onDragLeave"
      @drop.prevent="onDrop"
      @dragover.prevent
    >
      <label class="drop-prompt">
        <Icon :icon="faUpload" size="2x" />
        <span>{{ canDropFolders ? 'Drop files, folders or zip archives' : 'Drop files or zip archives' }}</span>
        <span class="text-k-fg-50 text-sm"
          >or click to choose. They wait here for review before joining the hoard.</span
        >
        <input
          :accept="acceptAttribute"
          class="sr-only"
          multiple
          name="file[]"
          type="file"
          @change="onFileInputChange"
        />
      </label>

      <section v-if="files.length" class="flex flex-col gap-3">
        <UploadSummary />
        <ul class="flex flex-col gap-1.5 max-h-80 overflow-y-auto">
          <li v-for="file in files" :key="file.id" class="h-9">
            <UploadItem :file class="h-full" data-testid="upload-item" />
          </li>
        </ul>
        <footer v-if="hasFailures" class="flex justify-end gap-2">
          <Btn size="small" variant="success" @click.prevent="retryAll">Retry failed</Btn>
          <Btn size="small" variant="destructive" @click.prevent="removeFailed">Remove failed</Btn>
        </footer>
      </section>

      <section v-if="batches.length" class="flex flex-col gap-4">
        <OfferingBatchCard
          v-for="batch in batches"
          :key="batch.batch"
          :batch
          @accept="accept(batch)"
          @discard="discard(batch)"
          @discard-file="discardFile"
        />
      </section>
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faUpload } from '@fortawesome/free-solid-svg-icons'
import { computed, onBeforeUnmount, onMounted, ref, toRef } from 'vue'
import { isDirectoryReadingSupported as canDropFolders } from '@/utils/supports'
import { acceptedExtensions } from '@/utils/mediaHelper'
import { pluralize } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { uploadService } from '@/services/uploadService'
import { huntingService } from '@/services/huntingService'
import type { OfferingBatch, OfferingFile } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { useUpload } from '@/composables/useUpload'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import UploadItem from '@/components/ui/upload/UploadItem.vue'
import UploadSummary from '@/components/ui/upload/UploadSummary.vue'
import OfferingBatchCard from '@/components/screens/hunting/OfferingBatchCard.vue'

const acceptAttribute = acceptedExtensions.map(ext => `.${ext}`).join(',')

const { allowsUpload, queueFilesForUpload, handleDropEvent } = useUpload()
const { showConfirmDialog } = useDialogBox()
const { toastSuccess, toastWarning } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const files = toRef(uploadService.state, 'files')
const hasFailures = computed(() => files.value.some(({ status }) => status === 'Errored' || status === 'Canceled'))
const batches = ref<OfferingBatch[]>([])
const droppable = ref(false)

const fetchBatches = async () => {
  try {
    batches.value = await huntingService.offerings()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const onDragEnter = () => (droppable.value = allowsUpload.value)

const onDragLeave = (e: MouseEvent) => {
  if ((e.currentTarget as Node)?.contains?.(e.relatedTarget as Node)) {
    return
  }

  droppable.value = false
}

const onDrop = async (event: DragEvent) => {
  droppable.value = false
  await handleDropEvent(event)
}

const onFileInputChange = (event: Event) => {
  const selected = (event.target as HTMLInputElement).files

  if (selected?.length) {
    queueFilesForUpload(Array.from(selected))
  }
}

const retryAll = () => uploadService.retryAll()
const removeFailed = () => uploadService.removeFailed()

const changed = async () => {
  await fetchBatches()
  await huntingStore.refresh()
}

const accept = async (batch: OfferingBatch) => {
  try {
    const { failures } = await huntingService.acceptBatch(batch.batch)

    if (failures.length) {
      toastWarning(`${pluralize(failures, 'file')} could not join the hoard: ${failures[0].error}`)
    } else {
      toastSuccess('Accepted. The new songs are in your library and being looked up on MusicBrainz.')
    }

    await changed()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const discard = async (batch: OfferingBatch) => {
  if (!(await showConfirmDialog(`Discard ${pluralize(batch.files, 'file')}?`))) {
    return
  }

  try {
    await huntingService.discardBatch(batch.batch)
    await changed()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const discardFile = async (file: OfferingFile) => {
  try {
    await huntingService.discardOffering(file.id)
    await changed()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchBatches)
eventBus.on('OFFERINGS_UPLOADED', changed)
onBeforeUnmount(() => eventBus.off('OFFERINGS_UPLOADED', changed))
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.drop-prompt {
  @apply flex flex-col items-center gap-2 p-10 rounded-3xl border-2 border-dashed border-k-fg-10 text-k-fg-70 cursor-pointer
    hover:border-k-fg-50 hover:text-k-fg transition;
}

.droppable .drop-prompt {
  @apply border-k-highlight bg-black/20 text-k-fg;
}
</style>
