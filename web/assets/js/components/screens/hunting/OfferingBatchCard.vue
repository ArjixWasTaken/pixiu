<template>
  <M3Card class="flex flex-col gap-2 p-4" variant="outlined">
    <header class="flex flex-wrap items-center gap-3">
      <div class="flex-1 min-w-0">
        <h3 class="m3-title-medium truncate">{{ title }}</h3>
        <p class="m3-body-medium muted">{{ origin }} · {{ pluralize(batch.files, 'file') }}</p>
      </div>
      <M3Button :disabled="!readable.length" @click.prevent="emit('accept')">
        Accept {{ readable.length === batch.files.length ? 'all' : readable.length }}
      </M3Button>
      <M3Button variant="text" @click.prevent="emit('discard')">Discard</M3Button>
    </header>

    <M3Divider />

    <ul class="flex flex-col">
      <li v-for="file in batch.files" :key="file.id" :class="{ unreadable: !file.readable }" class="file">
        <span class="m3-label-medium w-6 text-right muted tabular-nums">{{ file.track ?? '' }}</span>
        <div class="flex-1 min-w-0">
          <p :title="file.file_name" class="m3-body-medium truncate">{{ file.title }}</p>
          <p v-if="file.readable" class="m3-body-small truncate muted">
            {{ file.artist }} · {{ file.album }}<template v-if="file.year"> · {{ file.year }}</template>
          </p>
          <p v-else :title="file.error ?? ''" class="m3-body-small truncate error">Unreadable: {{ file.error }}</p>
        </div>
        <span class="m3-label-medium muted tabular-nums">{{ file.readable ? secondsToHis(file.length) : '' }}</span>
        <M3IconButton
          :icon-size="20"
          :label="`Discard ${file.file_name}`"
          icon="close"
          size="xs"
          @click.prevent="emit('discard-file', file)"
        />
      </li>
    </ul>
  </M3Card>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import type { OfferingBatch, OfferingFile } from '@/services/huntingService'
import { pluralize, secondsToHis } from '@/utils/formatters'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Divider from '@/components/m3/M3Divider.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'

const props = defineProps<{ batch: OfferingBatch }>()
const emit = defineEmits<{
  (e: 'accept'): void
  (e: 'discard'): void
  (e: 'discard-file', file: OfferingFile): void
}>()

const readable = computed(() => props.batch.files.filter(file => file.readable))

/** One album's files are named after it; else by how many albums they span. */
const title = computed(() => {
  const albums = new Set(readable.value.map(file => `${file.album}\u0000${file.album_artist ?? file.artist}`))

  if (albums.size === 1) {
    const first = readable.value[0]
    return `${first.album} — ${first.album_artist ?? first.artist}`
  }

  return albums.size ? `${albums.size} albums` : 'Unreadable files'
})

const origin = computed(() => {
  const archives = [...new Set(props.batch.files.map(file => file.archive).filter(Boolean))]
  return archives.length ? archives.join(', ') : 'Uploaded files'
})
</script>

<style scoped>
.file {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 0;
  list-style: none;
}

.muted {
  color: var(--schemes-on-surface-variant);
}

.error {
  color: var(--schemes-error);
}
</style>
