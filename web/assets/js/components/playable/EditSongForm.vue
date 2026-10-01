<template>
  <form class="form" @submit.prevent="handleSubmit" @keydown.esc="maybeClose">
    <header class="gap-4">
      <img :src="coverUrl" alt="" class="w-[84px] aspect-square object-cover object-center rounded-md" />
      <div class="flex-1 flex flex-col justify-center overflow-hidden">
        <h1 :class="{ mixed: editingMultipleSongs }">{{ displayedTitle }}</h1>
        <h2 :class="{ mixed: !allSongsAreFromSameArtist && !data.artist_name }" data-testid="displayed-artist-name">
          {{ displayedArtistName }}
        </h2>
        <h2 :class="{ mixed: !allSongsAreInSameAlbum && !data.album_name }" data-testid="displayed-album-name">
          {{ displayedAlbumName }}
        </h2>
      </div>
    </header>

    <M3Tabs v-if="editingOnlyOneSong" v-model="currentTab" :tabs class="tabs" secondary />

    <main class="pt-4">
      <div v-show="currentTab === 'details'" class="flex flex-col gap-4">
        <M3TextField
          v-if="editingOnlyOneSong"
          v-model="data.title"
          v-koel-focus
          data-testid="title-input"
          label="Title"
          name="title"
        />

        <div class="grid md:grid-cols-2 gap-4">
          <M3TextField
            v-model="data.artist_name"
            :placeholder="inputPlaceholder"
            data-testid="artist-input"
            label="Artist"
            name="artist"
          />
          <M3TextField
            v-model="data.album_artist_name"
            :placeholder="inputPlaceholder"
            data-testid="albumArtist-input"
            label="Album artist"
            name="album_artist"
          />
        </div>

        <M3TextField
          v-model="data.album_name"
          :placeholder="inputPlaceholder"
          data-testid="album-input"
          label="Album"
          name="album"
        />

        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <M3TextField
            v-model="data.track"
            :placeholder="inputPlaceholder"
            data-testid="track-input"
            label="Track"
            min="1"
            name="track"
            type="number"
          />
          <M3TextField
            v-model="data.disc"
            :placeholder="inputPlaceholder"
            data-testid="disc-input"
            label="Disc"
            min="1"
            name="disc"
            type="number"
          />
          <M3TextField
            v-model="data.genre"
            :placeholder="inputPlaceholder"
            data-testid="genre-input"
            label="Genre"
            list="genres"
            name="genre"
          />
          <M3TextField
            v-model="data.year"
            :placeholder="inputPlaceholder"
            data-testid="year-input"
            label="Year"
            name="year"
            type="number"
          />
        </div>
        <datalist id="genres">
          <option v-for="genre in genres" :key="genre" :value="genre" />
        </datalist>
      </div>

      <M3TextField
        v-if="editingOnlyOneSong"
        v-show="currentTab === 'lyrics'"
        v-model="data.lyrics"
        :rows="14"
        data-testid="lyrics-input"
        label="Lyrics"
        multiline
        name="lyrics"
      />
    </main>

    <footer>
      <M3Button class="btn-cancel" variant="text" @click.prevent="maybeClose">Cancel</M3Button>
      <M3Button type="submit">Save</M3Button>
    </footer>
  </form>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { pluralize } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import type { SongUpdateData, SongUpdateResult } from '@/stores/playableStore'
import { playableStore as songStore } from '@/stores/playableStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { genres } from '@/config/genres'
import { useForm } from '@/composables/useForm'
import { useBranding } from '@/composables/useBranding'

import M3Button from '@/components/m3/M3Button.vue'
import M3Tabs from '@/components/m3/M3Tabs.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = withDefaults(defineProps<{ songs: Song[]; initialTab?: EditSongFormTabName }>(), {
  initialTab: 'details',
})

const emit = defineEmits<{ (e: 'close'): void }>()
const songs = props.songs
const currentTab = ref<string>(props.initialTab)

const tabs = [
  { id: 'details', label: 'Details' },
  { id: 'lyrics', label: 'Lyrics' },
]

const close = () => emit('close')

const { toastSuccess } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { cover: defaultCover } = useBranding()

const editingOnlyOneSong = songs.length === 1
const editingMultipleSongs = !editingOnlyOneSong
const inputPlaceholder = editingMultipleSongs ? 'Leave unchanged' : ''

const allSongsShareSameValue = (key: keyof Song) =>
  editingMultipleSongs ? new Set(songs.map(song => song[key])).size === 1 : true

const allSongsAreFromSameArtist = allSongsShareSameValue('artist_name')
const allSongsAreInSameAlbum = allSongsShareSameValue('album_id')
const coverUrl = allSongsAreInSameAlbum ? songs[0].album_cover || defaultCover : defaultCover

const initialValues: SongUpdateData = {
  album_name: allSongsAreInSameAlbum ? songs[0].album_name : '',
  artist_name: allSongsAreFromSameArtist ? songs[0].artist_name : '',
  album_artist_name: '',
  track: allSongsShareSameValue('track') && songs[0].track !== 0 ? songs[0].track : null,
  disc: allSongsShareSameValue('disc') && songs[0].disc !== 0 ? songs[0].disc : null,
  year: allSongsShareSameValue('year') ? songs[0].year : null,
  genre: allSongsShareSameValue('genre') ? songs[0].genre : '',
  ...(editingOnlyOneSong
    ? {
        title: allSongsShareSameValue('title') ? songs[0].title : '',
        lyrics: editingOnlyOneSong ? songs[0].lyrics : '',
      }
    : {}),
}

if (allSongsAreInSameAlbum && allSongsAreFromSameArtist && songs[0].album_artist_id === songs[0].artist_id) {
  // If the album artist(s) is the same as the artist(s), we set the value as empty to not confuse the user
  // and make it less error-prone.
  initialValues.album_artist_name = ''
} else {
  initialValues.album_artist_name = allSongsShareSameValue('album_artist_name') ? songs[0].album_artist_name : ''
}

const { data, isPristine, handleSubmit } = useForm<SongUpdateData>({
  initialValues,
  onSubmit: async data => await songStore.updateSongs(songs, data),
  onSuccess: (result: SongUpdateResult) => {
    toastSuccess(`Updated ${pluralize(songs, 'song')}.`)
    eventBus.emit('SONGS_UPDATED', result)
    close()
  },
})

const displayedTitle = computed(() => (editingOnlyOneSong ? data.title : `${songs.length} songs selected`))

const displayedArtistName = computed(() => {
  return allSongsAreFromSameArtist || data.artist_name ? data.artist_name : 'Various artists'
})

const displayedAlbumName = computed(() =>
  allSongsAreInSameAlbum || data.album_name ? data.album_name : 'Various albums',
)

const maybeClose = async () => {
  if (isPristine() || (await showConfirmDialog('Discard all changes?'))) {
    close()
  }
}
</script>

<style scoped>
.form {
  width: min(560px, 100vw);
}

.tabs {
  border-bottom: 1px solid var(--schemes-outline-variant);
}

.mixed {
  color: var(--schemes-on-surface-variant);
}
</style>
