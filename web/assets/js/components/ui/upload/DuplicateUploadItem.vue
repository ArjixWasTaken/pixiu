<template>
  <article class="flex items-stretch min-h-[32px] bg-(--schemes-surface-container-high) rounded-lg overflow-hidden">
    <span
      class="self-center min-w-0 flex-1 overflow-hidden whitespace-nowrap px-4 [mask-image:linear-gradient(to_right,black_calc(100%-3rem),transparent)]"
    >
      {{ upload.song_title ? `${upload.artist_name} — ${upload.song_title}` : upload.filename }}
    </span>
    <time
      :datetime="upload.created_at"
      :title="uploadedAt.toLocaleString()"
      class="self-center shrink-0 px-4 text-(--schemes-on-surface-variant)"
    >
      Uploaded {{ uploadedAgo }}
    </time>
    <M3IconButton icon="check" label="Keep" @click="keep" />
    <M3IconButton icon="delete" label="Discard" @click="confirmDiscard" />
  </article>
</template>

<script setup lang="ts">
import { useTimeAgo } from '@vueuse/core'
import { useDialogBox } from '@/composables/useDialogBox'
import { uploadService } from '@/services/uploadService'

import M3IconButton from '@/components/m3/M3IconButton.vue'

import type { DuplicateUpload } from '@/services/uploadService'

const props = defineProps<{ upload: DuplicateUpload }>()

const uploadedAt = new Date(props.upload.created_at)
const uploadedAgo = useTimeAgo(uploadedAt)

const { showConfirmDialog } = useDialogBox()

const keep = () => uploadService.keepDuplicate(props.upload.id)

const confirmDiscard = async () => {
  if (await showConfirmDialog('Discard this duplicate upload?')) {
    uploadService.discardDuplicate(props.upload.id)
  }
}
</script>
