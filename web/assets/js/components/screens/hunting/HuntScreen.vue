<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Discover</ScreenHeader>
    </template>

    <div class="flex flex-col gap-6 pt-1" data-vue="HuntScreen">
      <form class="flex gap-3 items-center max-w-[720px]" @submit.prevent="handleSubmit">
        <M3TextField
          v-model="data.q"
          autofocus
          class="flex-1"
          label="Search YouTube Music"
          leading-icon="search"
          name="q"
          autocorrect="off"
          placeholder="Artist, album or song"
          required
          spellcheck="false"
          type="search"
        />
        <M3Button :disabled="searching" type="submit">Search</M3Button>
      </form>

      <div v-if="searching" class="flex items-center gap-3 text-(--schemes-on-surface-variant)">
        <M3ProgressIndicator :size="24" variant="circular" />
        <span class="m3-body-large">Searching YouTube Music…</span>
      </div>

      <template v-else-if="results">
        <ScreenEmptyState v-if="!results.albums.length && !results.tracks.length">
          <template #icon>
            <M3Icon :size="64" name="search_off" />
          </template>
          Nothing found for “{{ lastQuery }}”
          <span class="secondary block">Try the artist’s name alone, or check the spelling.</span>
        </ScreenEmptyState>

        <section v-if="results.albums.length">
          <h2 class="m3-title-large mb-3">Albums</h2>
          <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(170px,1fr))]">
            <HuntAlbumCard v-for="album in results.albums" :key="album.id" :album @grab="grabAlbum(album)" />
          </div>
        </section>

        <section v-if="results.tracks.length">
          <h2 class="m3-title-large mb-1">Songs</h2>
          <M3List class="-mx-3 py-0!">
            <HuntTrackRow v-for="track in results.tracks" :key="track.id" :track @grab="grabTrack(track)" />
          </M3List>
        </section>
      </template>

      <ScreenEmptyState v-else>
        <template #icon>
          <M3Icon :size="64" name="travel_explore" />
        </template>
        Find music on YouTube Music
        <span class="secondary block">Downloads show up on Jobs, then in your library.</span>
      </ScreenEmptyState>
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { HuntAlbum, HuntTrack } from '@/services/huntingService'
import { useForm } from '@/composables/useForm'
import { useRouter } from '@/composables/useRouter'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3List from '@/components/m3/M3List.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'
import M3TextField from '@/components/m3/M3TextField.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import HuntAlbumCard from '@/components/screens/hunting/HuntAlbumCard.vue'
import HuntTrackRow from '@/components/screens/hunting/HuntTrackRow.vue'

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const results = ref<{ tracks: HuntTrack[]; albums: HuntAlbum[] } | null>(null)
const searching = ref(false)
const lastQuery = ref('')

const { data, handleSubmit } = useForm<{ q: string }>({
  initialValues: { q: '' },
  useOverlay: false,
  validator: ({ q }) => q.trim() !== '',
  onSubmit: async ({ q }) => {
    searching.value = true
    lastQuery.value = q.trim()

    try {
      return await huntingService.search(lastQuery.value)
    } finally {
      searching.value = false
    }
  },
  onSuccess: found => (results.value = found),
})

const { getRouteParam, onScreenActivated } = useRouter()

// Library search sends its query along: `/discover?q=…`.
onScreenActivated('Hunt', () => {
  const q = getRouteParam('q')?.trim()

  if (q && q !== lastQuery.value) {
    data.q = q
    handleSubmit()
  }
})

const grab = async (item: HuntAlbum | HuntTrack, request: () => Promise<unknown>) => {
  try {
    await request()
    item.standing = 'pending'
    toastSuccess(`Downloading “${item.title}”. Jobs shows how it goes.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const grabAlbum = (album: HuntAlbum) => grab(album, () => huntingService.grabAlbum(album))
const grabTrack = (track: HuntTrack) => grab(track, () => huntingService.grabTrack(track))
</script>
