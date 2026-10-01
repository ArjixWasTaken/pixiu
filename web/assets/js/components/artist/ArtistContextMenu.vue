<template>
  <ul role="none">
    <SheetHeader :cover="artist.image" :title="artist.name" round />
    <MenuItem @click="play">Play all</MenuItem>
    <MenuItem @click="shuffle">Shuffle all</MenuItem>
    <Separator />
    <MenuItem @click="toggleFavorite">{{ artist.favorite ? 'Remove from favorites' : 'Add to favorites' }}</MenuItem>
    <RatingItem :rateable="artist" />
    <template v-if="allowEdit || (isStandardArtist && allowDownload) || musicBrainzUrl">
      <Separator />
      <MenuItem v-if="allowEdit" @click="requestEditForm">Edit…</MenuItem>
      <MenuItem v-if="isStandardArtist && allowDownload" @click="download">Download</MenuItem>
      <MenuItem v-if="musicBrainzUrl" @click="viewOnMusicBrainz">View on MusicBrainz</MenuItem>
    </template>
  </ul>
</template>

<script lang="ts" setup>
import { computed, toRef, toRefs } from 'vue'
import { useArtistStore } from '@/stores/artistStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useDownload } from '@/composables/useDownload'
import { defineAsyncComponent } from '@/utils/helpers'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { useRouter } from '@/composables/useRouter'
import { playback } from '@/services/playbackManager'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'

import RatingItem from '@/components/ui/context-menu/RatingItem.vue'
import SheetHeader from '@/components/ui/context-menu/SheetHeader.vue'

const artistStore = useArtistStore()
const commonStore = useCommonStore()
const playableStore = usePlayableStore()

const props = defineProps<{ artist: Artist }>()
const { artist } = toRefs(props)

const EditArtistForm = defineAsyncComponent(() => import('@/components/artist/EditArtistForm.vue'))

const { go, url } = useRouter()
const { MenuItem, Separator, trigger } = useContextMenu()
const { openModal } = useModal()

const allowDownload = toRef(commonStore.state, 'allows_download')
// Artists follow their albums' tags; píxiū has no artist editor.
const allowEdit = computed(() => false)

const isStandardArtist = computed(() => !artistStore.isUnknown(artist.value) && !artistStore.isVarious(artist.value))

const { useMusicBrainz } = useThirdPartyServices()

const musicBrainzUrl = computed(() =>
  useMusicBrainz.value && artist.value.mbid ? `https://musicbrainz.org/artist/${artist.value.mbid}` : null,
)

const viewOnMusicBrainz = () => trigger(() => window.open(musicBrainzUrl.value!, '_blank'))

const play = () =>
  trigger(async () => {
    go(url('queue'))
    await playback().queueAndPlay(await playableStore.fetchSongsForArtist(artist.value))
  })

const shuffle = () =>
  trigger(async () => {
    go(url('queue'))
    await playback().queueAndPlay(await playableStore.fetchSongsForArtist(artist.value), true)
  })

const { fromArtist } = useDownload()
const download = () => trigger(() => fromArtist(artist.value))
const toggleFavorite = () => trigger(() => artistStore.toggleFavorite(artist.value))
const requestEditForm = () => trigger(() => openModal<'EDIT_ARTIST_FORM'>(EditArtistForm, { artist: artist.value }))
</script>
