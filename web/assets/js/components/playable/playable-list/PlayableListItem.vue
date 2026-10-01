<template>
  <div class="px-3">
    <h4 v-if="showDisc && playable.disc" class="disc m3-title-small">Disc {{ playable.disc }}</h4>

    <article
      :class="{ playing, selected: item.selected }"
      class="song-item m3-state"
      data-testid="song-item"
      tabindex="0"
      @dblclick.prevent.stop="play"
    >
      <PlayableThumbnail :playable @clicked="play" />

      <span class="content">
        <span class="title m3-body-large">
          <M3Icon
            v-if="cachingOffline"
            :size="16"
            class="spin opacity-60"
            name="progress_activity"
            title="Caching for offline playback"
          />
          <M3Icon
            v-else-if="cachingFailed"
            :size="16"
            :title="`Error: ${cachingErrorMessage}`"
            class="text-(--schemes-error)"
            name="error"
          />
          <OfflineMark v-else-if="cachedOffline" />
          <span class="truncate">{{ playable.title }}</span>
        </span>
        <span class="supporting m3-body-medium">{{ supporting }}</span>
      </span>

      <span class="trailing">
        <span v-if="shouldShowColumn('rating')" class="rating">
          <StarRating :rateable="playable" size="xs" />
        </span>
        <span v-if="shouldShowColumn('duration')" class="time m3-label-medium">{{ fmtLength }}</span>
        <FavoriteButton v-if="shouldShowColumn('favorite')" :favorite="playable.favorite" @toggle="toggleFavorite" />
        <M3IconButton
          :icon-size="20"
          class="more"
          icon="more_vert"
          label="More actions"
          @click.stop="emit('request-context-menu', $event)"
        />
      </span>
    </article>
  </div>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { secondsToHis, timeAgo } from '@/utils/formatters'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { PlayableListConfigKey, PlayableListContextKey } from '@/config/symbols'
import { playableListColumnConfig } from '@/config/tables'
import { playableStore } from '@/stores/playableStore'
import { useViewport } from '@/composables/useViewport'

import PlayableThumbnail from '@/components/playable/PlayableThumbnail.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import OfflineMark from '@/components/ui/OfflineMark.vue'
import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import StarRating from '@/components/ui/StarRating.vue'

const props = withDefaults(defineProps<{ item: PlayableRow; showDisc?: boolean }>(), {
  showDisc: false,
})

const emit = defineEmits<{
  (e: 'play', playable: Playable): void
  (e: 'request-context-menu', event: MouseEvent): void
}>()

const [config] = requireInjection<[Partial<PlayableListConfig>]>(PlayableListConfigKey, [{}])
const [context] = requireInjection<[PlayableListContext]>(PlayableListContextKey, [{}])

const { shouldShowColumn } = useTableColumnVisibility(playableListColumnConfig)

const { item } = toRefs(props)

const playable = computed<Playable>(() => item.value.playable)
const playing = computed(() => ['Playing', 'Paused'].includes(playable.value.playback_state!))
const { isCached, isCaching, hasCachingError, getCachingError } = useOfflinePlayback()
const cachedOffline = computed(() => isCached(playable.value))
const cachingOffline = computed(() => isCaching(playable.value))
const cachingFailed = computed(() => hasCachingError(playable.value))
const cachingErrorMessage = computed(() => getCachingError(playable.value))

const fmtLength = secondsToHis(playable.value.length)
const artist = computed(() => playable.value.artist_name)
const album = computed(() => playable.value.album_name)

const { isMobile } = useViewport()

/** When it was played, on Recently played. */
const played = computed(() =>
  context.type === 'RecentlyPlayed' && playable.value.played_at ? `played ${timeAgo(playable.value.played_at)}` : null,
)

/** "Artist · album" on wide screens; phones show the length instead of the album. */
const supporting = computed(() =>
  [artist.value, isMobile.value ? fmtLength : shouldShowColumn('album') ? album.value : null, played.value]
    .filter(Boolean)
    .join(' · '),
)

const play = () => emit('play', playable.value)

const toggleFavorite = () => playableStore.toggleFavorite(playable.value)
</script>

<style lang="postcss" scoped>
.disc {
  padding: 16px 16px 8px;
  color: var(--schemes-on-surface-variant);
}

.song-item {
  display: flex;
  align-items: center;
  gap: 16px;
  height: 72px;
  padding: 8px 8px 8px 16px;
  border-radius: 16px;
  color: var(--schemes-on-surface);
  outline: none;

  &.selected {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }

  &.playing .title {
    color: var(--schemes-primary);
  }

  &:focus-visible {
    outline: 2px solid var(--schemes-secondary);
    outline-offset: -2px;
  }

  &.droppable {
    position: relative;
    transition: none;

    &::after {
      content: '';
      position: absolute;
      left: 16px;
      right: 16px;
      top: 0;
      height: 3px;
      border-radius: 2px;
      background: var(--schemes-primary);
    }

    &.dragover-bottom::after {
      top: auto;
      bottom: 0;
    }
  }
}

.content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.supporting {
  color: var(--schemes-on-surface-variant);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  .selected & {
    color: inherit;
  }
}

.trailing {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  color: var(--schemes-on-surface-variant);
}

.time {
  min-width: 44px;
  margin-right: 4px;
  text-align: right;
  font-variant-numeric: tabular-nums;

  @media (max-width: 768px) {
    display: none;
  }
}

.rating {
  margin-right: 8px;

  @media (max-width: 768px) {
    display: none;
  }
}

.spin {
  animation: m3-spin 1s linear infinite;
}
</style>
