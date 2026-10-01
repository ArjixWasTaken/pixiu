<template>
  <form class="md:w-[560px]" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header>
      <h1>Edit album</h1>
    </header>

    <main class="flex flex-col gap-4 pt-2">
      <M3TextField v-model="data.title" v-koel-focus label="Title" name="title" required />
      <div class="grid grid-cols-[1fr_8rem] gap-4">
        <M3TextField v-model="data.artist" label="Album artist" name="artist" required />
        <M3TextField v-model="data.year" label="Year" max="9999" min="1000" name="year" type="number" />
      </div>

      <section v-if="data.tracks.length" class="flex flex-col gap-2">
        <h2 class="m3-title-small text-(--schemes-on-surface)">Tracks</h2>
        <ul class="flex flex-col gap-3 max-h-72 overflow-y-auto pt-2">
          <li v-for="track in data.tracks" :key="track.id" class="grid grid-cols-[5rem_1fr] gap-2">
            <M3TextField v-model="track.track" :name="`track-${track.id}`" label="No." min="1" type="number" />
            <M3TextField v-model="track.title" :name="`title-${track.id}`" label="Title" required />
          </li>
        </ul>
      </section>
      <p v-else class="m3-body-medium text-(--schemes-on-surface-variant)">Loading tracks…</p>
      <p class="m3-body-small text-(--schemes-on-surface-variant)">
        This changes how píxiū lists the album; the files stay as they are.
      </p>
    </main>

    <footer>
      <M3Button variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button :disabled="!loaded" type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import { queryClient } from '@/services/queryClient'
import type { AlbumUpdateData } from '@/stores/albumStore'
import { useAlbumStore } from '@/stores/albumStore'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useDialogBox } from '@/composables/useDialogBox'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const albumStore = useAlbumStore()

const props = defineProps<{ album: Album }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { handleHttpError } = useErrorHandler('dialog')

const close = () => emit('close')
const loaded = ref(false)
let original = ''

const { data, handleSubmit } = useForm<AlbumUpdateData>({
  initialValues: { title: props.album.name, artist: props.album.artist_name, year: props.album.year, tracks: [] },
  // The form fills in once the tracks load; changes count from there.
  isPristine: (_, current) => JSON.stringify(current) === original,
  onSubmit: async form =>
    await albumStore.update(props.album, {
      ...form,
      year: form.year ? Number(form.year) : null,
      tracks: form.tracks.map(track => ({ ...track, track: track.track ? Number(track.track) : null })),
    }),
  onSuccess: () => {
    toastSuccess('Album updated.')
    close()
  },
})

const maybeClose = async () => {
  if (JSON.stringify(data) === original || (await showConfirmDialog('Discard all changes?', { action: 'Discard' }))) {
    close()
  }
}

onMounted(async () => {
  try {
    // Fresh for editing, and kept for the album's MusicBrainz panel.
    const details = await queryClient.fetchQuery({
      queryKey: ['album', props.album.id, 'details'],
      queryFn: () => huntingService.albumDetails(props.album),
      staleTime: 0,
    })
    Object.assign(data, {
      title: details.title,
      artist: details.artist,
      year: details.year,
      tracks: details.tracks.map(({ id, title, track }) => ({ id, title, track })),
    })
    original = JSON.stringify(data)
    loaded.value = true
  } catch (error: unknown) {
    handleHttpError(error)
    close()
  }
})
</script>
