<template>
  <div
    :class="{ round: !forAlbum }"
    :style="{ backgroundImage: `url(${defaultCover})` }"
    class="card-thumbnail"
    data-testid="album-artist-card-thumbnail"
  >
    <img v-if="image" :alt="entity.name" :src="image" loading="lazy" />

    <M3IconButton :label="playLabel" class="play" fill icon="play_arrow" variant="filled" @click.stop="playOrQueue" />
  </div>
</template>

<script lang="ts" setup>
import { orderBy } from 'lodash-es'
import { computed, toRefs } from 'vue'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useRouter } from '@/composables/useRouter'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { playback } from '@/services/playbackManager'
import { useBranding } from '@/composables/useBranding'
import { coverOfSize } from '@/services/subsonic'

import M3IconButton from '@/components/m3/M3IconButton.vue'

const queueStore = useQueueStore()
const playableStore = usePlayableStore()

const props = defineProps<{ entity: Album | Artist }>()
const { entity } = toRefs(props)

defineEmits<{
  (e: 'toggle-favorite'): void
  (e: 'context-menu', event: MouseEvent): void
}>()

const { toastSuccess } = useMessageToaster()
const { go, url } = useRouter()
const { cover: defaultCover } = useBranding()

const forAlbum = computed(() => entity.value.type === 'albums')

const image = computed(
  () =>
    coverOfSize(forAlbum.value ? (entity.value as Album).cover : (entity.value as Artist).image, 400) || defaultCover,
)

const playLabel = computed(() =>
  forAlbum.value ? `Play all songs in the album ${entity.value.name}` : `Play all songs by ${entity.value.name}`,
)

const sortFields = computed(() => (forAlbum.value ? ['disc', 'track'] : ['album_id', 'disc', 'track']))

const playOrQueue = async (event: MouseEvent) => {
  const songs = forAlbum.value
    ? await playableStore.fetchSongsForAlbum(entity.value as Album)
    : await playableStore.fetchSongsForArtist(entity.value as Artist)

  if (event.altKey) {
    queueStore.queue(orderBy(songs, sortFields.value))
    toastSuccess('Songs added to queue.')
    return
  }

  playback().queueAndPlay(songs)
  go(url('queue'))
}
</script>

<style scoped>
.card-thumbnail {
  position: relative;
  width: 100%;
  aspect-ratio: 1 / 1;
  overflow: hidden;
  border-radius: 8px;
  background-size: cover;
  background-position: center;

  &.round {
    border-radius: 50%;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.play {
  position: absolute;
  right: 8px;
  bottom: 8px;
  opacity: 0;
  box-shadow: var(--m3-elevation-2);
  transition: opacity 150ms linear;

  .round & {
    right: calc(50% - 20px);
    bottom: calc(50% - 20px);
  }

  .card-thumbnail:hover &,
  .card-thumbnail:focus-within & {
    opacity: 1;
  }

  @media (hover: none) {
    display: none;
  }
}
</style>
