<template>
  <ul role="none">
    <SheetHeader :cover="playlist.cover" :title="playlist.name" />
    <!-- An empty playlist has nothing to play, queue, download or keep offline. -->
    <template v-if="hasSongs">
      <MenuItem @click="play">Play</MenuItem>
      <MenuItem @click="shuffle">Shuffle</MenuItem>
      <MenuItem @click="addToQueue">Add to queue</MenuItem>
      <template v-if="allowDownload || canToggleOffline">
        <Separator />
        <MenuItem v-if="allowDownload" @click="download">Download</MenuItem>
        <MenuItem v-if="canToggleOffline" @click="toggleOffline">
          {{ allCached ? 'Remove offline copies' : 'Make available offline' }}
        </MenuItem>
      </template>
    </template>
    <template v-if="canMoveOutOfFolder || canEditPlaylist || canDeletePlaylist">
      <Separator v-if="hasSongs" />
      <MenuItem v-if="canMoveOutOfFolder" @click="moveOutOfFolder">Move out of folder</MenuItem>
      <MenuItem v-if="canEditPlaylist" @click="edit">Edit…</MenuItem>
      <MenuItem v-if="canDeletePlaylist" @click="destroy">Delete</MenuItem>
    </template>
  </ul>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, toRef, toRefs } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { defineAsyncComponent } from '@/utils/helpers'
import { pluralize } from '@/utils/formatters'
import { useRouter } from '@/composables/useRouter'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { usePolicies } from '@/composables/usePolicies'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { playback } from '@/services/playbackManager'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useDialogBox } from '@/composables/useDialogBox'
import { useCommonStore } from '@/stores/commonStore'
import { useDownload } from '@/composables/useDownload'

import SheetHeader from '@/components/ui/context-menu/SheetHeader.vue'

const queueStore = useQueueStore()
const playableStore = usePlayableStore()
const playlistFolderStore = usePlaylistFolderStore()
const playlistStore = usePlaylistStore()
const commonStore = useCommonStore()

const props = defineProps<{ playlist: Playlist }>()
const { playlist } = toRefs(props)

const EditPlaylistForm = defineAsyncComponent(() => import('@/components/playlist/EditPlaylistForm.vue'))
const EditSmartPlaylistForm = defineAsyncComponent(
  () => import('@/components/playlist/smart-playlist/EditSmartPlaylistForm.vue'),
)

const { MenuItem, Separator, trigger } = useContextMenu()
const { openModal } = useModal()
const { go, url } = useRouter()
const { toastWarning, toastSuccess } = useMessageToaster()
const { currentUserCan } = usePolicies()
const { showConfirmDialog } = useDialogBox()

const allowDownload = toRef(commonStore.state, 'allows_download')

const canEditPlaylist = computed(() => currentUserCan.editPlaylist(playlist.value))
const canDeletePlaylist = computed(() => currentUserCan.deletePlaylist(playlist.value))
const canMoveOutOfFolder = computed(() => playlist.value.folder_id !== null && canEditPlaylist.value)

const edit = () =>
  trigger(() => {
    const p = playlist.value
    p.is_smart
      ? openModal<'EDIT_SMART_PLAYLIST_FORM'>(EditSmartPlaylistForm, { playlist: p })
      : openModal<'EDIT_PLAYLIST_FORM'>(EditPlaylistForm, { playlist: p })
  })

const destroy = () =>
  trigger(async () => {
    if (await showConfirmDialog(`Delete the playlist "${playlist.value.name}"?`)) {
      await playlistStore.delete(playlist.value)
      toastSuccess(`Playlist "${playlist.value.name}" deleted.`)
      eventBus.emit('PLAYLIST_DELETED', playlist.value)
    }
  })

const { fromPlaylist } = useDownload()
const download = () => trigger(() => fromPlaylist(playlist.value))

const play = () =>
  trigger(async () => {
    const songs = await playableStore.fetchForPlaylist(playlist.value)

    if (songs.length) {
      playback().queueAndPlay(songs)
      go(url('queue'))
    } else {
      toastWarning('The playlist is empty.')
    }
  })

const shuffle = () =>
  trigger(async () => {
    const songs = await playableStore.fetchForPlaylist(playlist.value)

    if (songs.length) {
      playback().queueAndPlay(songs, true)
      go(url('queue'))
    } else {
      toastWarning('The playlist is empty.')
    }
  })

const addToQueue = () =>
  trigger(async () => {
    const songs = await playableStore.fetchForPlaylist(playlist.value)

    if (songs.length) {
      queueStore.queueAfterCurrent(songs)
      toastSuccess('Playlist added to queue.')
    } else {
      toastWarning('The playlist is empty.')
    }
  })

const moveOutOfFolder = () => trigger(() => playlistFolderStore.movePlaylistToFolder(playlist.value, null))

const { swReady, makePlayablesAvailableOffline, removePlayablesOfflineCache, allPlayablesCached } = useOfflinePlayback()
const canToggleOffline = computed(() => swReady.value)
const playlistSongs = ref<Playable[]>([])
const songsLoaded = ref(false)
const allCached = computed(() => allPlayablesCached(playlistSongs.value))

/** Until its songs are known, a playlist is taken to have some. */
const hasSongs = computed(() => !songsLoaded.value || playlistSongs.value.length > 0)

const toggleOffline = () =>
  trigger(async () => {
    if (!playlistSongs.value.length) return

    if (allCached.value) {
      removePlayablesOfflineCache(playlistSongs.value)
      toastSuccess(`Removed offline versions for "${playlist.value.name}".`)
    } else {
      makePlayablesAvailableOffline(playlistSongs.value)
      toastSuccess(`Making ${pluralize(playlistSongs.value, 'song')} available offline…`)
    }
  })

onMounted(async () => {
  playlistSongs.value = await playableStore.fetchForPlaylist(playlist.value)
  songsLoaded.value = true
})
</script>
