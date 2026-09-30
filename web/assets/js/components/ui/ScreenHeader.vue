<template>
  <header :class="[layout, { disabled, round: isArtist }]" class="screen-header">
    <aside v-if="$slots.thumbnail && layout === 'expanded'" class="thumbnail">
      <slot name="thumbnail" />
    </aside>

    <main class="body">
      <span v-if="layout === 'expanded' && label" class="m3-label-large overline-text">{{ label }}</span>
      <h1 :class="titleClass" class="name">
        <slot />
      </h1>
      <p v-if="$slots.description && layout === 'expanded'" class="description m3-body-large">
        <slot name="description" />
      </p>
      <p v-if="$slots.meta" class="meta m3-body-medium">
        <slot name="meta" />
      </p>
      <div v-if="$slots.controls" class="controls">
        <slot name="controls" />
      </div>
    </main>
  </header>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'

const props = withDefaults(
  defineProps<{
    layout?: ScreenHeaderLayout
    disabled?: boolean
    /** The small line above the title; derived from the screen when not given. */
    overline?: string
  }>(),
  {
    layout: 'expanded',
    disabled: false,
  },
)

const { getCurrentScreen } = useRouter()
const { isMobile } = useViewport()

const overlines: Partial<Record<ScreenName, string>> = {
  Songs: 'Library',
  Album: 'Album',
  Artist: 'Artist',
  Genre: 'Genre',
  Playlist: 'Playlist',
  Favorites: 'Playlist',
  RecentlyPlayed: 'Playlist',
  Queue: 'Now playing',
  Settings: 'Manage',
}

const label = computed(() => props.overline ?? overlines[getCurrentScreen()] ?? '')
const isArtist = computed(() => getCurrentScreen() === 'Artist')

const titleClass = computed(() => {
  if (props.layout === 'collapsed' || isMobile.value) {
    return 'm3-headline-medium'
  }

  return 'm3-display-small'
})
</script>

<style lang="postcss" scoped>
.screen-header {
  position: relative;
  display: flex;
  gap: 24px;
  align-items: flex-end;
  flex-shrink: 0;
  padding: 8px 24px 20px;
  color: var(--schemes-on-surface);

  &.collapsed {
    align-items: center;
    padding: 8px 24px 16px;
    min-height: 64px;

    .body {
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
      gap: 12px 16px;
    }

    .name {
      flex: 1;
      min-width: 200px;
    }

    .meta {
      order: 3;
      width: 100%;
      margin-top: -8px;
    }

    .controls {
      margin-top: 0;
    }
  }

  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;

    * {
      pointer-events: none;
    }
  }

  @media (max-width: 768px) {
    padding: 4px 16px 16px;

    &.collapsed {
      padding: 4px 16px 12px;

      .name {
        min-width: 100%;
        white-space: normal;
      }
    }
  }
}

.thumbnail {
  width: 180px;
  height: 180px;
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 28px;
  box-shadow: var(--m3-elevation-1);

  .round & {
    border-radius: 50%;
  }

  /* On phones, smaller: enough to recognize the album or artist by. */
  @media (max-width: 768px) {
    width: 96px;
    height: 96px;
    border-radius: 16px;
  }

  :deep(> *) {
    width: 100%;
    height: 100%;
    border-radius: 0;
  }
}

.body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.overline-text {
  color: var(--schemes-primary);
}

.name {
  margin: 0;
  color: var(--schemes-on-surface);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.description {
  margin: 0;
  color: var(--schemes-on-surface-variant);
}

.meta {
  margin: 0;
  color: var(--schemes-on-surface-variant);

  :deep(a) {
    color: inherit;

    &:hover {
      color: var(--schemes-primary);
    }
  }

  :deep(> * + *)::before {
    content: '·';
    margin: 0 6px;
  }
}

.controls {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
</style>
