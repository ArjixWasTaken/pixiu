<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader :layout="playables.length === 0 ? 'collapsed' : headerLayout">
        Current Queue

        <template #thumbnail>
          <ThumbnailStack :thumbnails />
        </template>

        <template v-if="playables.length" #meta>
          <span>{{ pluralize(playables, 'item') }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <PlayableListControls
            v-if="playables.length"
            :config
            @filter="applyFilter"
            @clear-queue="clearQueue"
            @play-all="playAll"
            @play-selected="playSelected"
          />
        </template>
      </ScreenHeader>
    </template>

    <PlayableListSkeleton v-if="loading" class="screen-bleed" role="status" aria-busy="true" aria-label="Loading" />
    <PlayableList
      v-if="playables.length"
      ref="playableList"
      class="screen-bleed"
      @reorder="onReorder"
      @press:delete="removeSelected"
      @press:enter="onPressEnter"
      @swipe="onSwipe"
    />

    <ScreenEmptyState v-else>
      <template #icon>
        <Icon :icon="faCoffee" />
      </template>

      No songs queued.
      <span v-if="libraryNotEmpty" class="block secondary">
        How about
        <a class="start" @click.prevent="shuffleSome">playing some random songs</a>?
      </span>
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { faCoffee } from '@fortawesome/free-solid-svg-icons'
import { computed, nextTick, onMounted, ref, toRef } from 'vue'
import { until } from '@vueuse/core'
import { pluralize } from '@/utils/formatters'
import { commonStore } from '@/stores/commonStore'
import { queueStore } from '@/stores/queueStore'
import { playableStore } from '@/stores/playableStore'
import { cache } from '@/services/cache'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { playback } from '@/services/playbackManager'
import { requireInjection } from '@/utils/helpers'
import { isRadioStation } from '@/utils/typeGuards'
import { CurrentStreamableKey } from '@/config/symbols'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'

const { go, onScreenActivated, url } = useRouter()

const {
  PlayableList,
  ThumbnailStack,
  headerLayout,
  playables,
  playableList,
  duration,
  thumbnails,
  selectedPlayables,
  playSelected,
  applyFilter,
  onSwipe,
} = usePlayableList(toRef(queueStore.state, 'playables'), { type: 'Queue' }, { reorderable: true, sortable: false })

const { PlayableListControls, config } = usePlayableListControls('Queue')

const loading = ref(false)
const libraryNotEmpty = computed(() => commonStore.state.song_count > 0)

const playAll = async (shuffle = true) => {
  playback().queueAndPlay(playables.value, shuffle)
  go(url('queue'))
}

const shuffleSome = async () => {
  try {
    loading.value = true
    await queueStore.fetchRandom()
    await playback().playFirstInQueue()
  } catch (error: unknown) {
    useErrorHandler('dialog').handleHttpError(error)
  } finally {
    loading.value = false
  }
}

const clearQueue = () => {
  playback().stop()
  queueStore.clear()
}

const removeSelected = async () => {
  if (!selectedPlayables.value.length) {
    return
  }

  const currentId = queueStore.current?.id
  queueStore.unqueue(selectedPlayables.value)

  if (currentId && selectedPlayables.value.find(({ id }) => id === currentId)) {
    await playback().playNext()
  }
}

const onPressEnter = () => selectedPlayables.value.length && playback().play(selectedPlayables.value[0])

const onReorder = (target: Playable, placement: Placement) =>
  queueStore.move(selectedPlayables.value, target, placement)

onScreenActivated('Queue', async () => {
  if (!cache.get('song-to-queue')) {
    return
  }

  let playable: Playable | undefined

  try {
    loading.value = true
    playable = await playableStore.resolve(cache.get('song-to-queue')!)

    if (!playable) {
      throw new Error('Song not found')
    }
  } catch (error: unknown) {
    useErrorHandler('dialog').handleHttpError(error)
    return
  } finally {
    cache.remove('playable-to-queue')
    loading.value = false
  }

  queueStore.clearSilently()
  queueStore.queue(playable!)
})

// What the player shows, playing or not: after a reload, the song the queue
// was saved at, before anything has played.
const currentStreamable = requireInjection(CurrentStreamableKey, ref())

/** The queue opens where it is playing, not at its top. */
const revealCurrent = async () => {
  const current = queueStore.current ?? currentStreamable.value

  if (!current || isRadioStation(current)) {
    return
  }

  // The list loads on its own, a moment after the screen.
  await until(() => Boolean(playableList.value)).toBe(true, { timeout: 5_000 })
  await nextTick()
  playableList.value?.scrollToPlayable(current)
}

// Shown again, and shown the first time (the route change that mounts the
// screen happens before it listens for one).
onScreenActivated('Queue', revealCurrent)
onMounted(revealCurrent)
</script>
