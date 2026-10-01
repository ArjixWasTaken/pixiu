<template>
  <article class="upload-item relative">
    <div
      :class="cssClass"
      class="h-full w-full min-h-[32px] bg-(--schemes-surface-container-high) relative rounded-lg overflow-hidden"
    >
      <div class="absolute z-1 h-full w-full flex items-center">
        <ProgressRing
          v-if="showsProgressRing"
          :aria-label="`Upload progress for ${file.name}`"
          :value="file.status === 'Ready' ? 0 : file.progress"
          class="size-4 shrink-0 ml-4"
          data-testid="upload-item-progress"
        />
        <a
          v-if="file.song"
          :href="url('albums.show', { id: file.song.album_id })"
          class="name min-w-0 flex-1 overflow-hidden whitespace-nowrap px-4 [mask-image:linear-gradient(to_right,black_calc(100%-3rem),transparent)] text-current focus:text-current"
          data-testid="upload-item-album-link"
        >
          {{ file.name }}
        </a>
        <span
          v-else
          class="name min-w-0 flex-1 overflow-hidden whitespace-nowrap px-4 [mask-image:linear-gradient(to_right,black_calc(100%-3rem),transparent)]"
          >{{ file.name }}</span
        >
        <span
          v-if="showsReasonInRow"
          class="min-w-0 truncate px-4 text-(--schemes-on-surface-variant)"
          data-testid="upload-item-reason"
        >
          {{ file.message }}
        </span>
        <span
          v-if="file.status === 'Retrying'"
          class="shrink-0 px-3 text-(--schemes-on-surface-variant)"
          data-testid="upload-item-state"
        >
          Retrying&hellip;
        </span>
        <M3IconButton v-if="canAbort" icon="close" label="Abort" @click="abort" />
        <M3IconButton v-if="canRetry" icon="refresh" label="Retry" @click="retry" />
        <M3IconButton v-if="canRemove" icon="delete" label="Remove" @click="remove" />
        <span v-if="file.status === 'Uploaded'" class="px-3 text-(--schemes-tertiary)" title="Uploaded">
          <M3Icon name="check_circle" />
        </span>
        <span v-if="isProcessing" class="px-3 text-(--schemes-on-surface-variant)" title="Processing">
          <M3Icon name="progress_activity" class="animate-spin" />
        </span>
      </div>
    </div>
  </article>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { useDialogBox } from '@/composables/useDialogBox'
import type { UploadFile } from '@/services/uploadService'
import { uploadService } from '@/services/uploadService'
import { useRouter } from '@/composables/useRouter'

import ProgressRing from '@/components/ui/ProgressRing.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'

const props = defineProps<{ file: UploadFile }>()

const { file } = toRefs(props)
const { url } = useRouter()

const isProcessing = computed(() => file.value.status === 'Processing')
const showsProgressRing = computed(() => ['Ready', 'Uploading', 'Retrying'].includes(file.value.status))
const canRetry = computed(() => file.value.status === 'Canceled' || file.value.status === 'Errored')
const canAbort = computed(() => file.value.status === 'Uploading')

const canRemove = computed(
  () => !['Uploading', 'Uploaded', 'Skipped'].includes(file.value.status) && !isProcessing.value,
)

const cssClass = computed(() => file.value.status.toLowerCase())
const showsReasonInRow = computed(() => ['Skipped', 'Errored', 'Canceled'].includes(file.value.status))

const progressBarWidth = computed(() => {
  if (isProcessing.value) {
    return '100%'
  }

  return file.value.status === 'Uploading' ? `${file.value.progress}%` : '0'
})

const { showConfirmDialog } = useDialogBox()

const remove = () => uploadService.remove(file.value)
const retry = () => uploadService.retry(file.value)

const abort = async () => {
  if ((await showConfirmDialog('Abort this upload?')) && file.value.status === 'Uploading') {
    uploadService.abort(file.value)
  }
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
article > div::before {
  width: v-bind(progressBarWidth);
  content: '';
  @apply absolute h-full top-0 left-0 z-0 duration-200 ease-out bg-(--schemes-primary);
}

.uploaded:hover {
  @apply bg-(--schemes-surface-container-highest);
}
</style>
