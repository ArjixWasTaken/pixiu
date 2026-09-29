<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Hunt</ScreenHeader>
    </template>

    <form class="flex gap-2 max-w-[720px] mb-8" @submit.prevent="handleSubmit">
      <TextInput
        v-model="data.q"
        v-koel-focus
        class="flex-1"
        name="q"
        placeholder="Artist, album or song on YouTube Music"
        required
        type="search"
      />
      <Btn :disabled="searching" type="submit">Search</Btn>
    </form>

    <p v-if="searching" class="text-k-fg-70">Searching YouTube Music…</p>

    <template v-else-if="results">
      <ScreenEmptyState v-if="!results.albums.length && !results.tracks.length">
        <template #icon>
          <Icon :icon="faSearch" />
        </template>
        Nothing found for “{{ lastQuery }}”
        <span class="secondary block">Try the artist’s name alone, or check the spelling.</span>
      </ScreenEmptyState>

      <section v-if="results.albums.length" class="mb-10">
        <h2 class="text-xl mb-4">Albums</h2>
        <div class="grid gap-5 grid-cols-[repeat(auto-fill,minmax(180px,1fr))]">
          <HuntAlbumCard v-for="album in results.albums" :key="album.id" :album @grab="grabAlbum(album)" />
        </div>
      </section>

      <section v-if="results.tracks.length">
        <h2 class="text-xl mb-4">Songs</h2>
        <ul class="divide-y divide-k-fg-5">
          <HuntTrackRow v-for="track in results.tracks" :key="track.id" :track @grab="grabTrack(track)" />
        </ul>
      </section>
    </template>

    <ScreenEmptyState v-else>
      <template #icon>
        <Icon :icon="faSearch" />
      </template>
      What should the beast bring back?
      <span class="secondary block">Search YouTube Music; grabbed songs show up on Jobs, then in your library.</span>
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faSearch } from '@fortawesome/free-solid-svg-icons'
import { ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { HuntAlbum, HuntTrack } from '@/services/huntingService'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import TextInput from '@/components/ui/form/TextInput.vue'
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

const grab = async (item: HuntAlbum | HuntTrack, request: () => Promise<unknown>) => {
  try {
    await request()
    item.standing = 'pending'
    toastSuccess(`“${item.title}” is on its way. Jobs shows how it goes.`)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const grabAlbum = (album: HuntAlbum) => grab(album, () => huntingService.grabAlbum(album))
const grabTrack = (track: HuntTrack) => grab(track, () => huntingService.grabTrack(track))
</script>
