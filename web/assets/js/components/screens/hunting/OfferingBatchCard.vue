<template>
  <article class="flex flex-col gap-3 p-4 rounded-lg bg-k-fg-5">
    <header class="flex flex-wrap items-center gap-3">
      <div class="flex-1 min-w-0">
        <h3 class="truncate text-lg">{{ title }}</h3>
        <p class="text-sm text-k-fg-70">{{ origin }} · {{ pluralize(batch.files, 'file') }}</p>
      </div>
      <Btn :disabled="!readable.length" size="small" variant="success" @click.prevent="emit('accept')">
        Accept {{ readable.length === batch.files.length ? 'all' : readable.length }}
      </Btn>
      <Btn size="small" variant="ghost" @click.prevent="emit('discard')">Discard</Btn>
    </header>

    <ul class="divide-y divide-k-fg-5 text-sm">
      <li
        v-for="file in batch.files"
        :key="file.id"
        :class="{ unreadable: !file.readable }"
        class="flex items-center gap-3 py-1.5"
      >
        <span class="w-8 text-right text-k-fg-50 tabular-nums">{{ file.track ?? '' }}</span>
        <div class="flex-1 min-w-0">
          <p :title="file.file_name" class="truncate">{{ file.title }}</p>
          <p v-if="file.readable" class="truncate text-k-fg-50">
            {{ file.artist }} · {{ file.album }}<template v-if="file.year"> · {{ file.year }}</template>
          </p>
          <p v-else :title="file.error ?? ''" class="truncate error">Unreadable: {{ file.error }}</p>
        </div>
        <span class="text-k-fg-50 tabular-nums">{{ file.readable ? secondsToHis(file.length) : '' }}</span>
        <button
          :title="`Discard ${file.file_name}`"
          class="text-k-fg-50 hover:text-k-danger px-1"
          type="button"
          @click.prevent="emit('discard-file', file)"
        >
          <Icon :icon="faTimes" />
        </button>
      </li>
    </ul>
  </article>
</template>

<script lang="ts" setup>
import { faTimes } from '@fortawesome/free-solid-svg-icons'
import { computed } from 'vue'
import type { OfferingBatch, OfferingFile } from '@/services/huntingService'
import { pluralize, secondsToHis } from '@/utils/formatters'

import Btn from '@/components/ui/form/Btn.vue'

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

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.unreadable .error {
  @apply text-k-danger;
}
</style>
