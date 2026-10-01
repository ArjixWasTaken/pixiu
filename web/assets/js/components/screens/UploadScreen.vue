<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        Uploads
        <template #meta>
          <span>{{ batches.length ? `${pluralize(batches, 'upload')} to review` : 'Nothing to review' }}</span>
        </template>
      </ScreenHeader>
    </template>

    <div
      :class="{ droppable }"
      class="flex flex-col gap-6 pt-1"
      data-vue="UploadScreen"
      @dragenter.prevent="onDragEnter"
      @dragleave.prevent="onDragLeave"
      @drop.prevent="onDrop"
      @dragover.prevent
    >
      <label class="drop-prompt">
        <M3Icon :size="36" name="upload" />
        <span class="m3-title-medium text-(--schemes-on-surface)">
          {{ canDropFolders ? 'Drop files, folders or zip archives' : 'Drop files or zip archives' }}
        </span>
        <span class="m3-body-medium">Or click to choose. They wait here for review before joining your library.</span>
        <input
          :accept="acceptAttribute"
          class="sr-only"
          multiple
          name="file[]"
          type="file"
          @change="onFileInputChange"
        />
      </label>

      <section v-if="files.length" class="flex flex-col gap-2">
        <UploadSummary />
        <ul class="flex flex-col gap-1.5 max-h-80 overflow-y-auto">
          <li v-for="file in files" :key="file.id" class="h-9">
            <UploadItem :file class="h-full" data-testid="upload-item" />
          </li>
        </ul>
        <footer v-if="hasFailures" class="flex justify-end gap-2">
          <M3Button icon="refresh" variant="tonal" @click.prevent="retryAll">Retry failed</M3Button>
          <M3Button class="text-(--schemes-error)!" variant="text" @click.prevent="removeFailed"
            >Remove failed</M3Button
          >
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
import { useQuery } from '@tanstack/vue-query'
import { computed, onBeforeUnmount, ref, toRef, watch } from 'vue'
import { isDirectoryReadingSupported as canDropFolders } from '@/utils/supports'
import { acceptedExtensions } from '@/utils/mediaHelper'
import { pluralize } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { uploadService } from '@/services/uploadService'
import { huntingService } from '@/services/huntingService'
import type { OfferingBatch, OfferingFile } from '@/services/huntingService'
import { useHuntingStore } from '@/stores/huntingStore'
import { useUpload } from '@/composables/useUpload'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import UploadItem from '@/components/ui/upload/UploadItem.vue'
import UploadSummary from '@/components/ui/upload/UploadSummary.vue'
import OfferingBatchCard from '@/components/screens/hunting/OfferingBatchCard.vue'

const huntingStore = useHuntingStore()

const acceptAttribute = acceptedExtensions.map(ext => `.${ext}`).join(',')

const { allowsUpload, queueFilesForUpload, handleDropEvent } = useUpload()
const { showConfirmDialog } = useDialogBox()
const { toastSuccess, toastWarning } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const files = toRef(uploadService.state, 'files')
const hasFailures = computed(() => files.value.some(({ status }) => status === 'Errored' || status === 'Canceled'))
const droppable = ref(false)

const { data, error, refetch } = useQuery({
  queryKey: ['hunting', 'offerings'],
  queryFn: () => huntingService.offerings(),
})
watch(error, error => error && handleHttpError(error))

const batches = computed(() => data.value ?? [])

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
  await refetch()
  await huntingStore.refresh()
}

const accept = async (batch: OfferingBatch) => {
  try {
    const { failures } = await huntingService.acceptBatch(batch.batch)

    if (failures.length) {
      toastWarning(`${pluralize(failures, 'file')} could not be added: ${failures[0].error}`)
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

eventBus.on('OFFERINGS_UPLOADED', changed)
onBeforeUnmount(() => eventBus.off('OFFERINGS_UPLOADED', changed))
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.drop-prompt {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px 24px;
  border: 2px dashed var(--schemes-outline-variant);
  border-radius: 28px;
  background: var(--schemes-surface-container-low);
  color: var(--schemes-on-surface-variant);
  text-align: center;
  cursor: pointer;
  transition:
    border-color 150ms linear,
    background-color 150ms linear;

  &:hover {
    border-color: var(--schemes-outline);
  }
}

.droppable .drop-prompt {
  border-color: var(--schemes-primary);
  background: var(--schemes-secondary-container);
}
</style>
