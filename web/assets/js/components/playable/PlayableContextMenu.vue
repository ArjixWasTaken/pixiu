<template>
  <ul>
    <template v-if="asSheet">
      <li class="sheet-header" tabindex="-1">
        <span
          :style="{ backgroundImage: `url(${sheetSong!.album_cover}), url(${defaultCover})` }"
          class="sheet-cover"
        />
        <span class="flex-1 min-w-0">
          <span class="m3-title-medium block truncate">{{ sheetSong!.title }}</span>
          <span class="m3-body-medium block truncate text-(--schemes-on-surface-variant)">{{ sheetSubtitle }}</span>
        </span>
        <FavoriteButton :favorite="sheetSong!.favorite" size="md" @toggle="toggleSheetFavorite" />
        <M3IconButton icon="close" label="Close" @click.stop="closeContextMenu" />
      </li>
      <li class="separator" />
      <li class="sheet-tiles" tabindex="-1">
        <button type="button" @click.stop="queueAfterCurrent">
          <span class="tile"><M3Icon :size="26" name="queue_play_next" /></span>
          <span class="m3-label-large">Play next</span>
        </button>
        <button type="button" @click.stop="showPlaylists = !showPlaylists">
          <span class="tile"><M3Icon :size="26" name="playlist_add" /></span>
          <span class="m3-label-large">Save to playlist</span>
        </button>
        <button v-if="canBeShared" type="button" @click.stop="copyUrl">
          <span class="tile"><M3Icon :size="26" name="share" /></span>
          <span class="m3-label-large">Share</span>
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
      <li class="sheet-rating" tabindex="-1">
        <M3Icon class="text-(--schemes-on-surface-variant)" name="star" />
        <span class="flex-1">Rating</span>
        <StarRating :rateable="sheetSong!" @rate="closeContextMenu" />
      </li>
    </template>

    <template v-if="onlyOneSelected">
      <MenuItem @click="doPlayback">{{ firstSongPlaying ? 'Pause' : 'Play' }}</MenuItem>
      <Separator />
      <MenuItem>
        Go to
        <template #subMenuItems>
          <template v-if="isSong(playables[0])">
            <MenuItem :title="playables[0].album_name" @click="viewAlbum(playables[0] as Song)">
              <template #icon>
                <Icon :icon="faCompactDisc" fixed-width />
              </template>
              {{ playables[0].album_name }}
            </MenuItem>
            <MenuItem :title="playables[0].artist_name" @click="viewArtist(playables[0] as Song)">
              <template #icon>
                <MicVocalIcon :size="16" class="inline-block" />
              </template>
              {{ playables[0].artist_name }}
            </MenuItem>
          </template>
          <template v-else>
            <MenuItem @click="viewPodcast(playables[0] as Episode)">
              <template #icon>
                <Icon :icon="faPodcast" fixed-width />
              </template>
              Podcast
            </MenuItem>
            <MenuItem @click="viewEpisode(playables[0] as Episode)">
              <template #icon>
                <Icon :icon="faHeadphones" fixed-width />
              </template>
              Episode
            </MenuItem>
            <MenuItem
              v-if="(playables[0] as Episode).episode_link"
              @click="visitEpisodeWebpage(playables[0] as Episode)"
            >
              <template #icon>
                <Icon :icon="faExternalLink" fixed-width />
              </template>
              Webpage
            </MenuItem>
          </template>
        </template>
      </MenuItem>
    </template>
    <MenuItem>
      Add To
      <template #subMenuItems>
        <template v-if="queue.length">
          <MenuItem v-if="currentSong" @click="queueAfterCurrent">After Current</MenuItem>
          <MenuItem @click="queueToBottom">Bottom of Queue</MenuItem>
          <MenuItem @click="queueToTop">Top of Queue</MenuItem>
        </template>
        <MenuItem v-else @click="queueToBottom">Queue</MenuItem>
        <template v-if="!isFavoritesScreen && !(onlyOneSelected && playables[0].favorite)">
          <Separator />
          <MenuItem @click="addToFavorites">Favorites</MenuItem>
        </template>
        <Separator v-if="normalPlaylists.length" />
        <template class="block">
          <ul v-if="normalPlaylists.length" class="scroll-mask-y relative max-h-48 overflow-y-auto">
            <MenuItem v-for="p in normalPlaylists" :key="p.id" @click="addToExistingPlaylist(p)">
              {{ p.name }}
            </MenuItem>
          </ul>
        </template>
        <Separator />
        <MenuItem @click="addToNewPlaylist">New Playlist…</MenuItem>
      </template>
    </MenuItem>

    <template v-if="onlyOneSelected && isSong(playables[0]) && !asSheet">
      <Separator />
      <li
        tabindex="-1"
        class="px-4 py-2 focus:outline-hidden"
        @mouseover="($event.currentTarget as HTMLLIElement).focus()"
      >
        <StarRating :rateable="playables[0] as Song" @rate="closeContextMenu" />
      </li>
      <Separator />
    </template>

    <template v-if="isQueueScreen">
      <Separator />
      <MenuItem @click="removeFromQueue">Remove from Queue</MenuItem>
      <Separator />
    </template>

    <template v-if="isFavoritesScreen">
      <Separator />
      <MenuItem @click="removeFromFavorites">Remove from Favorites</MenuItem>
    </template>

    <template v-if="visibilityActions.length">
      <Separator />
      <MenuItem v-for="{ label, handler } in visibilityActions" :key="label" @click="handler">
        {{ label }}
      </MenuItem>
    </template>

    <MenuItem v-if="canShare">
      Share
      <template #subMenuItems>
        <MenuItem v-if="canBeShared" @click="copyUrl">
          <template #icon>
            <Icon :icon="faLink" fixed-width />
          </template>
          Copy URL
        </MenuItem>
        <MenuItem v-if="allowEmbedding" @click="showEmbedModal">
          <template #icon>
            <Icon :icon="faCode" fixed-width />
          </template>
          Embed…
        </MenuItem>
      </template>
    </MenuItem>

    <MenuItem v-if="onlyOneSelected && isSong(playables[0])" @click="openSongInfo">Song Info…</MenuItem>
    <MenuItem v-if="downloadable" @click="download">Download</MenuItem>
    <MenuItem v-if="canToggleOffline" @click="toggleOffline">
      {{ allCached ? 'Remove Offline Versions' : 'Make Available Offline' }}
    </MenuItem>

    <template v-if="canBeRemovedFromPlaylist">
      <Separator />
      <MenuItem @click="removePlayablesFromPlaylist">Remove from Playlist</MenuItem>
    </template>

    <template v-if="mirroredWatch && contentType === 'songs'">
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
import {
  faCode,
  faCompactDisc,
  faExternalLink,
  faHeadphones,
  faLink,
  faPodcast,
} from '@fortawesome/free-solid-svg-icons'
import { MicVocalIcon } from 'lucide-vue-next'
import { computed, ref, toRef, toRefs } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { pluralize, secondsToHis } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { copyText } from '@/utils/helpers'
import { getPlayableCollectionContentType, isSong } from '@/utils/typeGuards'
import { commonStore } from '@/stores/commonStore'
import { playlistStore } from '@/stores/playlistStore'
import { queueStore } from '@/stores/queueStore'
import { playableStore } from '@/stores/playableStore'
import { useDownload } from '@/composables/useDownload'
import { useRouter } from '@/composables/useRouter'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useDialogBox } from '@/composables/useDialogBox'
import { usePlaylistContentManagement } from '@/composables/usePlaylistContentManagement'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'
import { usePlayableMenuMethods } from '@/composables/usePlayableMenuMethods'
import { usePolicies } from '@/composables/usePolicies'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { useKoelPlus } from '@/composables/useKoelPlus'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { playback } from '@/services/playbackManager'
import { huntingStore } from '@/stores/huntingStore'

import { useViewport } from '@/composables/useViewport'
import { useBranding } from '@/composables/useBranding'

import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import StarRating from '@/components/ui/StarRating.vue'

const props = defineProps<{ playables: Playable[] }>()
const { playables } = toRefs(props)

const { toastSuccess, toastError, toastWarning } = useMessageToaster()
const { showConfirmDialog } = useDialogBox()
const { go, getRouteParam, isCurrentScreen, url } = useRouter()
const SongInfo = defineAsyncComponent(() => import('@/components/playable/SongInfo.vue'))
const CreateEmbedForm = defineAsyncComponent(() => import('@/components/embed/CreateEmbedForm.vue'))

const { MenuItem, Separator, closeContextMenu, trigger } = useContextMenu()
const { openModal } = useModal()
const { removeFromPlaylist } = usePlaylistContentManagement()
const { isPlus } = useKoelPlus()

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

const { currentUserCan } = usePolicies()

const contentType = computed(() => getPlayableCollectionContentType(playables.value))
const allowEdit = computed(() => contentType.value === 'songs' && currentUserCan.editSong(playables.value as Song[]))
const onlyOneSelected = computed(() => playables.value.length === 1)

// On phones, a single song gets the design's sheet: a header, quick actions and its rating.
const { isMobile } = useViewport()
const { cover: defaultCover } = useBranding()
const showPlaylists = ref(false)
const sheetSong = computed(() => (onlyOneSelected.value && isSong(playables.value[0]) ? playables.value[0] : null))
const asSheet = computed(() => isMobile.value && Boolean(sheetSong.value))
const sheetSubtitle = computed(() =>
  sheetSong.value ? `${sheetSong.value.artist_name} · ${secondsToHis(sheetSong.value.length)}` : '',
)
const toggleSheetFavorite = () => sheetSong.value && playableStore.toggleFavorite(sheetSong.value)

const { useMusicBrainz } = useThirdPartyServices()

const musicBrainzUrl = computed(() => {
  if (!useMusicBrainz.value || !onlyOneSelected.value || !isSong(playables.value[0])) {
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
const canBeShared = computed(() => !isPlus.value || (isSong(playables.value[0]) && playables.value[0].is_public))
const allowEmbedding = toRef(commonStore.state, 'allows_embedding')
const canShare = computed(() => onlyOneSelected.value && (canBeShared.value || allowEmbedding.value))

const makePublic = () =>
  trigger(async () => {
    if (contentType.value !== 'songs') {
      throw new Error('Only songs can be marked as public or private')
    }

    await playableStore.publicizeSongs(playables.value as Song[])
    toastSuccess(`Unmarked ${pluralize(playables.value, 'song')} as private.`)
  })

const makePrivate = () =>
  trigger(async () => {
    if (contentType.value !== 'songs') {
      throw new Error('Only songs can be marked as public or private')
    }

    const privatizedIds = await playableStore.privatizeSongs(playables.value as Song[])

    if (!privatizedIds.length) {
      toastError('Songs cannot be marked as private if they’re part of a collaborative playlist.')
      return
    }

    if (privatizedIds.length < playables.value.length) {
      toastWarning('Some songs cannot be marked as private as they’re part of a collaborative playlist.')
      return
    }

    toastSuccess(`Marked ${pluralize(playables.value, 'song')} as private.`)
  })

const visibilityActions = computed(() => {
  if (contentType.value !== 'songs' || !allowEdit.value) {
    return []
  }

  if (!isPlus.value) {
    return []
  }

  const visibilities = Array.from(
    new Set((playables.value as Song[]).map(song => (song.is_public ? 'public' : 'private'))),
  )

  if (visibilities.length === 2) {
    return [
      {
        label: 'Unmark as Private',
        handler: makePublic,
      },
      {
        label: 'Mark as Private',
        handler: makePrivate,
      },
    ]
  }

  return visibilities[0] === 'public'
    ? [{ label: 'Mark as Private', handler: makePrivate }]
    : [{ label: 'Unmark as Private', handler: makePublic }]
})

const canBeRemovedFromPlaylist = computed(() => {
  if (!isCurrentScreen('Playlist')) {
    return false
  }
  const playlist = playlistStore.byId(getRouteParam('id')!)
  return playlist && !playlist.is_smart && playlist.permissions.edit
})

const isQueueScreen = computed(() => isCurrentScreen('Queue'))
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
const viewPodcast = (episode: Episode) => trigger(() => go(url('podcasts.show', { id: episode.podcast_id })))
const viewEpisode = (episode: Episode) => trigger(() => go(url('episodes.show', { id: episode.id })))
const visitEpisodeWebpage = (episode: Episode) => trigger(() => window.open(episode.episode_link!, '_blank'))
const { fromPlayables } = useDownload()
const download = () => trigger(() => fromPlayables(playables.value))

const { swReady, makeAvailableOffline, removeOfflineCache, isCached } = useOfflinePlayback()
const canToggleOffline = computed(() => contentType.value === 'songs' && swReady.value)
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

const copyUrl = () =>
  trigger(async () => {
    await copyText(playableStore.getShareableUrl(playables.value[0]))
    toastSuccess('URL copied to clipboard.')
  })

const showEmbedModal = () =>
  trigger(() => openModal<'CREATE_EMBED_FORM'>(CreateEmbedForm, { embeddable: playables.value[0] }))
</script>

<style scoped>
.sheet-header {
  gap: 12px !important;
  padding: 0 8px 12px 20px !important;
  cursor: default !important;

  &:hover {
    background: transparent !important;
  }
}

.sheet-cover {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  background-size: cover;
  background-position: center;
}

.sheet-tiles {
  display: grid !important;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px !important;
  padding: 16px 16px 12px !important;
  cursor: default !important;

  &:hover {
    background: transparent !important;
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

.sheet-rating {
  cursor: default !important;

  &:hover {
    background: transparent !important;
  }
}
</style>
