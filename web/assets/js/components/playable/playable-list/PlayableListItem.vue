<template>
  <div class="px-3">
    <h4 v-if="showDisc && playable.disc" class="disc m3-title-small">Disc {{ playable.disc }}</h4>

    <article
      ref="article"
      :class="{ playing, selected: item.selected }"
      class="song-item m3-state"
      data-testid="song-item"
      tabindex="0"
      @dblclick.prevent.stop="play"
    >
      <PlayableThumbnail :numbered="inAlbum" :playable @clicked="play" />

      <span class="content">
        <span class="title m3-body-large truncate">{{ playable.title }}</span>
        <span v-if="item.selected" class="sr-only">Selected</span>
        <span class="supporting m3-body-medium">
          <span class="supporting-text">{{ supporting }}</span>
          <!-- Phones show the length here; it never gives way to the rest. -->
          <span v-if="isMobile" class="supporting-length">{{ supporting ? '\u00a0· ' : '' }}{{ fmtLength }}</span>
        </span>
      </span>

      <span class="trailing">
        <span v-if="shouldShowColumn('rating')" class="rating">
          <StarRating :rateable="playable" size="xs" />
        </span>
        <span v-if="shouldShowColumn('duration')" class="time m3-label-medium">{{ fmtLength }}</span>
        <OfflineButton :class="{ reveal: offlineIdle }" :playable />
        <M3IconButton
          :icon-size="20"
          class="more reveal"
          icon="more_vert"
          label="More actions"
          @click.stop="emit('request-context-menu', $event)"
        />
      </span>
    </article>
  </div>
</template>

<script lang="ts" setup>
import { onLongPress, useEventListener } from '@vueuse/core'
import { computed, toRefs, useTemplateRef } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { secondsToHis, timeAgo } from '@/utils/formatters'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { PlayableListConfigKey, PlayableListContextKey } from '@/config/symbols'
import { playableListColumnConfig } from '@/config/tables'
import { useViewport } from '@/composables/useViewport'

import PlayableThumbnail from '@/components/playable/PlayableThumbnail.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import OfflineButton from '@/components/ui/OfflineButton.vue'
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
const { isCached, isCaching, hasCachingError } = useOfflinePlayback()
/** Neither kept offline nor on its way: the button shows only with the row's other actions. */
const offlineIdle = computed(
  () => !isCached(playable.value) && !isCaching(playable.value) && !hasCachingError(playable.value),
)

const fmtLength = secondsToHis(playable.value.length)
const artist = computed(() => playable.value.artist_name)
const album = computed(() => playable.value.album_name)

const { isMobile, isTouch } = useViewport()

/** In an album, the cover is the album's: rows show their track number instead. */
const inAlbum = computed(() => context.type === 'Album')

/** When it was played, on Recently played. */
const played = computed(() =>
  context.type === 'RecentlyPlayed' && playable.value.played_at ? `played ${timeAgo(playable.value.played_at)}` : null,
)

/** "Artist · album" on wide screens; phones show the length (after this) instead of the album. */
const supporting = computed(() =>
  [artist.value, !isMobile.value && shouldShowColumn('album') && !inAlbum.value ? album.value : null, played.value]
    .filter(Boolean)
    .join(' · '),
)

const play = () => emit('play', playable.value)

const article = useTemplateRef('article')

// On a phone, a long press opens the song's menu, as ⋮ does (Android would show
// its own menu; iOS nothing); the tap that ends it plays nothing.
let longPressed = false

onLongPress(
  article,
  event => {
    if (!isTouch.value) {
      return
    }

    longPressed = true
    emit('request-context-menu', event as MouseEvent)
  },
  { delay: 500, distanceThreshold: 10 },
)

useEventListener(
  article,
  'click',
  (event: MouseEvent) => {
    if (longPressed) {
      longPressed = false
      event.stopImmediatePropagation()
      event.preventDefault()
    }
  },
  { capture: true },
)
</script>

<style lang="postcss" scoped>
.disc {
  display: flex;
  align-items: flex-end;
  height: var(--m3-disc-height);
  padding: 0 16px 8px;
  color: var(--schemes-on-surface-variant);
}

.song-item {
  display: flex;
  align-items: center;
  gap: 16px;
  height: var(--m3-row-height);
  /* The cover lines up with the screen's content (the row's background reaches
     a little past it, into the screen's padding). */
  padding: 0 8px 0 calc(var(--screen-pad-x, 24px) - 12px);
  border-radius: 12px;
  color: var(--schemes-on-surface);
  outline: none;

  &.selected {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);

    .title {
      color: var(--schemes-on-surface);
    }
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
  min-width: 0;
}

.supporting {
  display: flex;
  min-width: 0;
  color: var(--schemes-on-surface-variant);
  white-space: nowrap;

  .selected & {
    color: inherit;
  }
}

.supporting-text {
  overflow: hidden;
  text-overflow: ellipsis;
}

.supporting-length {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
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

  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    display: none;
  }
}

.rating {
  margin-right: 8px;

  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    display: none;
  }
}

/* A long press opens the song's menu, not the browser's text selection. */
@media (hover: none) {
  .song-item {
    -webkit-touch-callout: none;
    user-select: none;
  }
}

/* With a mouse, a row's actions show when it is pointed at, focused or selected. */
@media (hover: hover) {
  .reveal {
    opacity: 0;
    transition: opacity 100ms linear;

    .song-item:is(:hover, :focus-within, .selected) & {
      opacity: 1;
    }
  }
}
</style>
