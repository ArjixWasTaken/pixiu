<template>
  <ul role="none">
    <MenuItem @click="play">Play all</MenuItem>
    <MenuItem @click="shuffle">Shuffle all</MenuItem>
    <Separator />
    <MenuItem @click="toggleFavorite">{{ album.favorite ? 'Remove from favorites' : 'Add to favorites' }}</MenuItem>
    <Separator />
    <li
      tabindex="-1"
      class="px-4 py-2 focus:outline-hidden"
      @mouseover="($event.currentTarget as HTMLLIElement).focus()"
    >
      <StarRating :rateable="album" @rate="closeContextMenu" />
    </li>
    <Separator />
    <template v-if="allowEdit">
      <MenuItem @click="edit">Edit…</MenuItem>
    </template>
    <Separator />
    <template v-if="isStandardAlbum && allowDownload">
      <MenuItem @click="download">Download</MenuItem>
    </template>
    <template v-if="canToggleOffline">
      <Separator />
      <MenuItem @click="toggleOffline">{{ allCached ? 'Remove offline copies' : 'Make available offline' }}</MenuItem>
    </template>
    <template v-if="musicBrainzUrl">
      <Separator />
      <MenuItem @click="viewOnMusicBrainz">View on MusicBrainz</MenuItem>
    </template>
  </ul>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, toRef, toRefs } from 'vue'
import { useAlbumStore } from '@/stores/albumStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useDownload } from '@/composables/useDownload'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { pluralize } from '@/utils/formatters'
import { defineAsyncComponent } from '@/utils/helpers'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { usePolicies } from '@/composables/usePolicies'
import { useRouter } from '@/composables/useRouter'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'
import { playback } from '@/services/playbackManager'

import StarRating from '@/components/ui/StarRating.vue'

const albumStore = useAlbumStore()
const commonStore = useCommonStore()
const playableStore = usePlayableStore()

const props = defineProps<{ album: Album }>()
const { album } = toRefs(props)

const EditAlbumForm = defineAsyncComponent(() => import('@/components/album/EditAlbumForm.vue'))

const { go, url } = useRouter()
const { MenuItem, Separator, closeContextMenu, trigger } = useContextMenu()
const { openModal } = useModal()
const { currentUserCan } = usePolicies()

const allowDownload = toRef(commonStore.state, 'allows_download')
const allowEdit = computed(() => currentUserCan.editAlbum(album.value))

const isStandardAlbum = computed(() => !albumStore.isUnknown(album.value))

const { useMusicBrainz } = useThirdPartyServices()

const musicBrainzUrl = computed(() =>
  useMusicBrainz.value && album.value.mbid ? `https://musicbrainz.org/release/${album.value.mbid}` : null,
)

const viewOnMusicBrainz = () => trigger(() => window.open(musicBrainzUrl.value!, '_blank'))

const play = () =>
  trigger(async () => {
    go(url('queue'))
    await playback().queueAndPlay(await playableStore.fetchSongsForAlbum(album.value))
  })

const shuffle = () =>
  trigger(async () => {
    go(url('queue'))
    await playback().queueAndPlay(await playableStore.fetchSongsForAlbum(album.value), true)
  })

const edit = () => trigger(() => openModal<'EDIT_ALBUM_FORM'>(EditAlbumForm, { album: album.value }))
const toggleFavorite = () => trigger(() => albumStore.toggleFavorite(album.value))
const { fromAlbum } = useDownload()
const download = () => trigger(() => fromAlbum(album.value))

const { swReady, makePlayablesAvailableOffline, removePlayablesOfflineCache, allPlayablesCached } = useOfflinePlayback()
const canToggleOffline = computed(() => swReady.value)
const albumSongs = ref<Playable[]>([])
const allCached = computed(() => allPlayablesCached(albumSongs.value))

const toggleOffline = () =>
  trigger(async () => {
    const { toastSuccess } = useMessageToaster()
    if (!albumSongs.value.length) return

    if (allCached.value) {
      removePlayablesOfflineCache(albumSongs.value)
      toastSuccess(`Removed offline versions for "${album.value.name}".`)
    } else {
      makePlayablesAvailableOffline(albumSongs.value)
      toastSuccess(`Making ${pluralize(albumSongs.value, 'song')} available offline…`)
    }
  })

onMounted(async () => {
  albumSongs.value = await playableStore.fetchSongsForAlbum(album.value)
})
</script>
