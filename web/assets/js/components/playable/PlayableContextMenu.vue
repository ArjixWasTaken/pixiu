<template>
  <ul role="none">
    <template v-if="asSheet">
      <SheetHeader :cover="sheetSong!.album_cover" :subtitle="sheetSubtitle" :title="sheetSong!.title">
        <FavoriteButton :favorite="sheetSong!.favorite" size="md" @toggle="toggleSheetFavorite" />
      </SheetHeader>
      <li class="sheet-tiles" tabindex="-1">
        <!-- Not for the song playing: it can't come after itself. -->
        <button v-if="canPlayNext" type="button" @click.stop="queueAfterCurrent">
          <span class="tile"><M3Icon :size="26" name="queue_play_next" /></span>
          <span class="m3-label-large">Play next</span>
        </button>
        <button type="button" @click.stop="showPlaylists = !showPlaylists">
          <span class="tile"><M3Icon :size="26" name="playlist_add" /></span>
          <span class="m3-label-large">Add to playlist</span>
        </button>
      </li>
      <template v-if="showPlaylists">
        <MenuItem v-for="p in normalPlaylists" :key="p.id" @click="addToExistingPlaylist(p)">
          <template #icon><M3Icon name="queue_music" /></template>
          {{ p.name }}
        </MenuItem>
        <MenuItem @click="addToNewPlaylist">
          <template #icon><M3Icon name="add" /></template>
          New playlist…
        </MenuItem>
        <Separator />
      </template>
      <RatingItem :rateable="sheetSong!" />
    </template>

    <template v-if="onlyOneSelected">
      <MenuItem @click="doPlayback">{{ firstSongPlaying ? 'Pause' : 'Play' }}</MenuItem>
      <Separator />
      <MenuItem>
        Go to
        <template #subMenuItems>
          <MenuItem :title="playables[0].album_name" @click="viewAlbum(playables[0])">
            <template #icon>
              <M3Icon name="album" />
            </template>
            Album: {{ playables[0].album_name }}
          </MenuItem>
          <MenuItem :title="playables[0].artist_name" @click="viewArtist(playables[0])">
            <template #icon>
              <M3Icon name="artist" :size="16" class="inline-block" />
            </template>
            Artist: {{ playables[0].artist_name }}
          </MenuItem>
        </template>
      </MenuItem>
    </template>
    <MenuItem>
      Add to
      <template #subMenuItems>
        <template v-if="queue.length">
          <MenuItem v-if="currentSong" @click="queueAfterCurrent">After current song</MenuItem>
          <MenuItem @click="queueToBottom">Bottom of queue</MenuItem>
          <MenuItem @click="queueToTop">Top of queue</MenuItem>
        </template>
        <MenuItem v-else @click="queueToBottom">Queue</MenuItem>
        <template v-if="!isFavoritesScreen && !(onlyOneSelected && playables[0].favorite)">
          <Separator />
          <MenuItem @click="addToFavorites">Favorites</MenuItem>
        </template>
        <Separator v-if="normalPlaylists.length" />
        <template class="block">
          <ul v-if="normalPlaylists.length" class="scroll-mask-y relative max-h-48 overflow-y-auto" role="none">
            <MenuItem v-for="p in normalPlaylists" :key="p.id" @click="addToExistingPlaylist(p)">
              {{ p.name }}
            </MenuItem>
          </ul>
        </template>
        <Separator />
        <MenuItem @click="addToNewPlaylist">New playlist…</MenuItem>
      </template>
    </MenuItem>

    <template v-if="onlyOneSelected && !asSheet">
      <Separator />
      <RatingItem :rateable="playables[0]" />
      <Separator />
    </template>

    <template v-if="isQueueScreen">
      <Separator />
      <MenuItem @click="removeFromQueue">Remove from queue</MenuItem>
      <Separator />
    </template>

    <template v-if="isFavoritesScreen">
      <Separator />
      <MenuItem @click="removeFromFavorites">Remove from favorites</MenuItem>
    </template>

    <MenuItem v-if="onlyOneSelected" @click="openSongInfo">Song info…</MenuItem>
    <MenuItem v-if="downloadable" @click="download">Download</MenuItem>
    <MenuItem v-if="canToggleOffline" @click="toggleOffline">
      {{ allCached ? 'Remove offline copies' : 'Make available offline' }}
    </MenuItem>

    <template v-if="canBeRemovedFromPlaylist">
      <Separator />
      <MenuItem @click="removePlayablesFromPlaylist">Remove from playlist</MenuItem>
    </template>

    <template v-if="mirroredWatch">
      <Separator />
      <MenuItem @click="excludeFromWatch">Exclude from “{{ mirroredWatch.name }}”</MenuItem>
    </template>

    <template v-if="musicBrainzUrl">
      <Separator />
      <MenuItem @click="viewOnMusicBrainz">View on MusicBrainz</MenuItem>
    </template>
  </ul>
</template>

<script lang="ts" setup>
import { computed, ref, toRef, toRefs } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { pluralize, secondsToHis } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { useCommonStore } from '@/stores/commonStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useDownload } from '@/composables/useDownload'
import { useRouter } from '@/composables/useRouter'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useDialogBox } from '@/composables/useDialogBox'
import { usePlaylistContentManagement } from '@/composables/usePlaylistContentManagement'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'
import { usePlayableMenuMethods } from '@/composables/usePlayableMenuMethods'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { playback } from '@/services/playbackManager'
import { useHuntingStore } from '@/stores/huntingStore'
import { useViewport } from '@/composables/useViewport'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import RatingItem from '@/components/ui/context-menu/RatingItem.vue'
import SheetHeader from '@/components/ui/context-menu/SheetHeader.vue'

const commonStore = useCommonStore()
const playlistStore = usePlaylistStore()
const queueStore = useQueueStore()
const playableStore = usePlayableStore()
const huntingStore = useHuntingStore()

const props = withDefaults(
  defineProps<{
    playables: Playable[]
    /** Opened from the queue itself (Up next): its songs can come out of it. */
    fromQueue?: boolean
  }>(),
  { fromQueue: false },
)
const { playables } = toRefs(props)

const { toastSuccess, toastError } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { go, getRouteParam, isCurrentScreen, url } = useRouter()
const SongInfo = defineAsyncComponent(() => import('@/components/playable/SongInfo.vue'))

const { MenuItem, Separator, closeContextMenu, trigger } = useContextMenu()
const { openModal } = useModal()
const { removeFromPlaylist } = usePlaylistContentManagement()

const {
  queueAfterCurrent,
  queueToBottom,
  queueToTop,
  addToFavorites,
  addToExistingPlaylist,
  removeFromFavorites,
  removeFromQueue,
  addToNewPlaylist,
} = usePlayableMenuMethods(playables, closeContextMenu)

const playlists = toRef(playlistStore.state, 'playlists')

const downloadable = computed(() => {
  if (!commonStore.state.allows_download) {
    return false
  }

  // If multiple playables are selected, make sure the zip extension is available on the server
  return playables.value.length === 1 || commonStore.state.supports_batch_downloading
})

const queue = toRef(queueStore.state, 'playables')
const currentSong = computed(() => queueStore.current)

const onlyOneSelected = computed(() => playables.value.length === 1)

// On phones, a single song gets the design's sheet: a header, quick actions and its rating.
const { isMobile } = useViewport()
const showPlaylists = ref(false)
const sheetSong = computed(() => (onlyOneSelected.value ? playables.value[0] : null))

/** "Play next" needs something playing, other than the song itself. */
const canPlayNext = computed(() => Boolean(currentSong.value) && currentSong.value?.id !== sheetSong.value?.id)
const asSheet = computed(() => isMobile.value && Boolean(sheetSong.value))
const sheetSubtitle = computed(() =>
  sheetSong.value ? `${sheetSong.value.artist_name} · ${secondsToHis(sheetSong.value.length)}` : '',
)
const toggleSheetFavorite = () => sheetSong.value && playableStore.toggleFavorite(sheetSong.value)

const { useMusicBrainz } = useThirdPartyServices()

const musicBrainzUrl = computed(() => {
  if (!useMusicBrainz.value || !onlyOneSelected.value) {
    return null
  }

  const { mbid } = playables.value[0] as Song

  return mbid ? `https://musicbrainz.org/recording/${mbid}` : null
})

const viewOnMusicBrainz = () => trigger(() => window.open(musicBrainzUrl.value!, '_blank'))
const firstSongPlaying = computed(() =>
  playables.value.length ? playables.value[0].playback_state === 'Playing' : false,
)
// Mirrors of watched playlists change on YouTube Music only.
const normalPlaylists = computed(() =>
  playlists.value.filter(({ is_smart, permissions }) => !is_smart && permissions.edit),
)
const canBeRemovedFromPlaylist = computed(() => {
  if (!isCurrentScreen('Playlist')) {
    return false
  }
  const playlist = playlistStore.byId(getRouteParam('id')!)
  return playlist && !playlist.is_smart && playlist.permissions.edit
})

const isQueueScreen = computed(() => props.fromQueue || isCurrentScreen('Queue'))
const isFavoritesScreen = computed(() => isCurrentScreen('Favorites'))

const doPlayback = () =>
  trigger(async () => {
    if (!playables.value.length) {
      return
    }

    switch (playables.value[0].playback_state) {
      case 'Playing':
        await playback().pause()
        break

      case 'Paused':
        await playback().resume()
        break

      default:
        await playback().play(playables.value[0])
        break
    }
  })

const openSongInfo = () => trigger(() => openModal<'SONG_INFO'>(SongInfo, { song: playables.value[0] as Song }))

/** On a mirror of a watched playlist, the watch its songs can be excluded from. */
const mirroredWatch = computed(() =>
  isCurrentScreen('Playlist') ? (huntingStore.playlistWatches[getRouteParam('id')!]?.watch ?? null) : null,
)

const excludeFromWatch = () =>
  trigger(async () => {
    const watch = mirroredWatch.value!

    try {
      await huntingStore.exclude(watch.id, playables.value as Song[])
      toastSuccess(`Excluded ${pluralize(playables.value, 'song')} from “${watch.name}”.`)
    } catch (error: unknown) {
      toastError('Excluding failed.')
      throw error
    }
  })

const viewAlbum = (song: Song) => trigger(() => go(url('albums.show', { id: song.album_id })))
const viewArtist = (song: Song) => trigger(() => go(url('artists.show', { id: song.artist_id })))
const { fromPlayables } = useDownload()
const download = () => trigger(() => fromPlayables(playables.value))

const { swReady, makeAvailableOffline, removeOfflineCache, isCached } = useOfflinePlayback()
const canToggleOffline = computed(() => swReady.value)
const allCached = computed(() => playables.value.every(p => isCached(p)))

const toggleOffline = () =>
  trigger(() => {
    if (allCached.value) {
      playables.value.forEach(p => removeOfflineCache(p))
      toastSuccess(
        playables.value.length === 1
          ? 'Removed offline version.'
          : `Removed ${playables.value.length} offline versions.`,
      )
    } else {
      playables.value.filter(p => !isCached(p)).forEach(p => makeAvailableOffline(p))
      toastSuccess(`Making ${pluralize(playables.value, 'song')} available offline…`)
    }
  })

const removePlayablesFromPlaylist = () =>
  trigger(async () => {
    const playlist = playlistStore.byId(getRouteParam('id'))

    if (!playlist) {
      return
    }

    await removeFromPlaylist(playlist, playables.value)
  })
</script>

<style scoped>
.sheet-tiles {
  display: grid !important;
  /* The tiles there are share the row. */
  grid-auto-columns: minmax(0, 1fr);
  grid-auto-flow: column;
  gap: 12px !important;
  padding: 16px 16px 12px !important;
  cursor: default !important;

  @media (hover: hover) {
    &:hover {
      background: transparent !important;
    }
  }

  button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    color: var(--schemes-on-surface);
  }

  .tile {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 64px;
    border-radius: 16px;
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}
</style>
