<template>
  <span
    role="radiogroup"
    :aria-label="`Rating: ${currentRating} of 5 stars`"
    class="inline-flex items-center gap-0.5"
    @mouseleave="hover = 0"
  >
    <label
      v-for="star in 5"
      :key="star"
      :title="titleFor(star)"
      :class="{ lit: (hover || currentRating) >= star }"
      class="star cursor-pointer transition-[color] duration-150"
      @click="onClick($event, star)"
      @mouseenter="hover = star"
    >
      <input
        type="radio"
        :name="groupName"
        :value="star"
        :checked="currentRating === star"
        class="sr-only"
        @change="onChange(star)"
      />
      <M3Icon :fill="(hover || currentRating) >= star" :size="size === 'xs' ? 16 : 20" name="star" />
      <span class="sr-only">Rate {{ star }} of 5</span>
    </label>
  </span>
</template>

<script lang="ts" setup>
import type { Reactive } from 'vue'
import { computed, ref, useId } from 'vue'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { usePlayableStore } from '@/stores/playableStore'
import M3Icon from '@/components/m3/M3Icon.vue'

const albumStore = useAlbumStore()
const artistStore = useArtistStore()
const playableStore = usePlayableStore()

type Rateable = Song | Album | Artist

const props = withDefaults(
  defineProps<{
    rateable?: Rateable
    rating?: number
    size?: 'xs' | 'sm'
  }>(),
  { size: 'sm' },
)

const emit = defineEmits<{ (e: 'rate', value: number): void }>()

const hover = ref(0)
const groupName = `rating-${useId()}`

const currentRating = computed(() => props.rateable?.rating ?? props.rating ?? 0)

const titleFor = (star: number) =>
  currentRating.value === star ? 'Remove rating' : `${star} star${star === 1 ? '' : 's'}`

const persist = (entity: Rateable, value: number) => {
  if (entity.type === 'songs') {
    return playableStore.rate(entity as Reactive<Song>, value)
  }

  if (entity.type === 'albums') {
    return albumStore.rate(entity as Reactive<Album>, value)
  }

  return artistStore.rate(entity as Reactive<Artist>, value)
}

const dispatch = (value: number) => {
  if (props.rateable) {
    persist(props.rateable, value)
  }
  emit('rate', value)
}

const onChange = (value: number) => dispatch(value)

const onClick = (event: MouseEvent, star: number) => {
  // A radio can't deselect itself on a normal click, so intercept clicks on the
  // currently-active star and dispatch 0 (clear) instead of letting the input fire change.
  if (currentRating.value === star) {
    event.preventDefault()
    dispatch(0)
  }
}
</script>

<style scoped>
/* The outline color reads in light and dark schemes alike. */
.star {
  color: var(--schemes-outline);

  &.lit {
    color: var(--schemes-primary);
  }
}
</style>
