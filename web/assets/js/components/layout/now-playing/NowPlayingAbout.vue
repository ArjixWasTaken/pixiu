<template>
  <div class="now-playing-about">
    <M3SegmentedButton v-model="nowPlaying.about.value" :segments class="self-start" />

    <div v-if="entity" class="flex gap-4 items-center">
      <div
        :class="{ round: nowPlaying.about.value === 'Artist' }"
        :style="{ backgroundImage: `url(${image}), url(${defaultCover})` }"
        class="art"
      />
      <a :href="entityUrl" class="m3-headline-small entity-link">{{ entity.name }}</a>
    </div>

    <ParagraphSkeleton v-if="loading" />
    <div v-else-if="text" class="m3-body-large text" v-html="text" />
    <p v-else class="m3-body-large text">píxiū knows nothing about {{ entity?.name ?? 'this' }} yet.</p>

    <M3Button v-if="source" :href="source" class="self-start" rel="noopener" target="_blank" variant="text"
      >Source</M3Button
    >
  </div>
</template>

<script lang="ts" setup>
import { computed, ref, toRefs, watch } from 'vue'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { encyclopediaService } from '@/services/encyclopediaService'
import { useBranding } from '@/composables/useBranding'
import { useNowPlaying } from '@/composables/useNowPlaying'
import { useRouter } from '@/composables/useRouter'

import M3Button from '@/components/m3/M3Button.vue'
import M3SegmentedButton from '@/components/m3/M3SegmentedButton.vue'
import ParagraphSkeleton from '@/components/ui/ParagraphSkeleton.vue'

const albumStore = useAlbumStore()
const artistStore = useArtistStore()

const props = defineProps<{ song: Song }>()
const { song } = toRefs(props)

const nowPlaying = useNowPlaying()
const { cover: defaultCover } = useBranding()

const segments = [
  { id: 'Artist', label: 'Artist' },
  { id: 'Album', label: 'Album' },
]

const artist = ref<Artist>()
const album = ref<Album>()
const artistInfo = ref<ArtistInfo | null>(null)
const albumInfo = ref<AlbumInfo | null>(null)
const loading = ref(false)

const entity = computed(() => (nowPlaying.about.value === 'Artist' ? artist.value : album.value))

const { url } = useRouter()

/** The artist's or album's own page. */
const entityUrl = computed(() => {
  if (!entity.value) {
    return undefined
  }
  return nowPlaying.about.value === 'Artist'
    ? url('artists.show', { id: entity.value.id })
    : url('albums.show', { id: entity.value.id })
})

const image = computed(() =>
  nowPlaying.about.value === 'Artist'
    ? artistInfo.value?.image || artist.value?.image || defaultCover
    : albumInfo.value?.cover || album.value?.cover || defaultCover,
)

const text = computed(() =>
  nowPlaying.about.value === 'Artist' ? artistInfo.value?.bio?.full : albumInfo.value?.wiki?.full,
)

const source = computed(() => (nowPlaying.about.value === 'Artist' ? artistInfo.value?.url : albumInfo.value?.url))

const load = async () => {
  loading.value = true

  try {
    if (nowPlaying.about.value === 'Artist') {
      artist.value = await artistStore.resolve(song.value.artist_id)
      artistInfo.value = artist.value ? await encyclopediaService.fetchForArtist(artist.value) : null
    } else {
      album.value = await albumStore.resolve(song.value.album_id)
      albumInfo.value = album.value ? await encyclopediaService.fetchForAlbum(album.value) : null
    }
  } finally {
    loading.value = false
  }
}

watch([song, nowPlaying.about], load, { immediate: true })
</script>

<style scoped>
.entity-link {
  color: var(--schemes-on-surface);

  @media (hover: hover) {
    &:hover {
      text-decoration: underline;
    }
  }
}

.now-playing-about {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 16px 24px;
  color: var(--schemes-on-surface);
}

.art {
  width: 88px;
  height: 88px;
  flex-shrink: 0;
  border-radius: 12px;
  background-size: cover;
  background-position: center;

  &.round {
    border-radius: 50%;
  }
}

.text {
  color: var(--schemes-on-surface-variant);
}
</style>
