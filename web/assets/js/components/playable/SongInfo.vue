<template>
  <div class="max-w-[560px]" tabindex="0" @keydown.esc="close">
    <header class="gap-4">
      <img :src="song.album_cover || defaultCover" alt="" class="w-[84px] aspect-square object-cover rounded-xl" />
      <div class="flex-1 flex flex-col justify-center overflow-hidden">
        <h1 class="truncate">{{ song.title }}</h1>
        <h2 class="m3-body-medium truncate text-(--schemes-on-surface-variant)">{{ song.artist_name }}</h2>
        <h2 class="m3-body-medium truncate text-(--schemes-on-surface-variant)">{{ song.album_name }}</h2>
      </div>
    </header>

    <main class="flex flex-col gap-6">
      <M3ProgressIndicator v-if="!info" />

      <template v-else>
        <dl class="m3-body-medium grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5">
          <dt>Format</dt>
          <dd>{{ info.format }} · {{ formatBytes(info.size) }}</dd>
          <dt>Came from</dt>
          <dd>
            <a v-if="info.youtube_url" :href="info.youtube_url" rel="noopener" target="_blank">YouTube Music</a>
            <template v-else>
              Upload<template v-if="info.source_name"> “{{ info.source_name }}”</template>
              <template v-if="info.source_archive"> from {{ info.source_archive }}</template>
            </template>
            · {{ timeAgo(info.added_at) }}
          </dd>
          <dt>Lyrics</dt>
          <dd>{{ lyricsLabel }}</dd>
          <template v-if="info.mbid">
            <dt>MusicBrainz</dt>
            <dd>
              <a :href="`https://musicbrainz.org/recording/${info.mbid}`" rel="noopener" target="_blank">Recording</a>
            </dd>
          </template>
        </dl>

        <section>
          <h3 class="m3-title-small text-(--schemes-primary) mb-2">Why píxiū keeps it</h3>
          <p v-if="!info.kept.length" class="text-(--schemes-error)">Nothing keeps it: it is an orphan.</p>
          <ul class="flex flex-col gap-2">
            <li v-for="(reason, index) in info.kept" :key="index" class="flex items-center gap-3">
              <span class="flex-1">{{ reason.why }}</span>
              <Btn
                v-if="reason.excludable_from"
                size="small"
                variant="ghost"
                @click.prevent="exclude(reason.excludable_from)"
              >
                Exclude
              </Btn>
            </li>
          </ul>
          <p v-if="info.kept.some(reason => reason.excludable_from)" class="text-xs text-k-fg-50 mt-2">
            Excluding a song from a watched playlist takes it out of the mirror; the watch won’t fetch it again.
          </p>
        </section>
      </template>
    </main>

    <footer>
      <Btn @click.prevent="close">Close</Btn>
    </footer>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, toRefs } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { SongInfo } from '@/services/huntingService'
import { huntingStore } from '@/stores/huntingStore'
import { formatBytes, timeAgo } from '@/utils/formatters'
import { useBranding } from '@/composables/useBranding'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

const props = defineProps<{ song: Song }>()
const emit = defineEmits<{ (e: 'close'): void }>()
const { song } = toRefs(props)

const { cover: defaultCover } = useBranding()
const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const info = ref<SongInfo | null>(null)

const lyricsLabel = computed(
  () =>
    ({
      file: 'From the file',
      lrclib: 'From LRCLIB',
      youtube_music: 'From YouTube Music',
      instrumental: 'None: marked instrumental',
      missing: 'None found',
      not_looked_up: 'Not looked up yet',
    })[info.value!.lyrics] ?? info.value!.lyrics,
)

const close = () => emit('close')

const fetchInfo = async () => {
  try {
    info.value = await huntingService.songInfo(song.value)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const exclude = async (watchId: number) => {
  try {
    await huntingStore.exclude(watchId, [song.value])
    toastSuccess(`Excluded “${song.value.title}”. The watch lets go of it and won’t fetch it again.`)
    await fetchInfo()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchInfo)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

dt {
  @apply text-k-fg-70;
}
</style>
