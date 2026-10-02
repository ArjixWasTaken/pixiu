<template>
  <div class="add-to w-full max-w-[256px] min-w-[200px] p-3 space-y-3" data-testid="add-to-menu">
    <ul v-if="config.queue" class="space-y-1.5" @keydown.enter.space.prevent="choose">
      <!-- With nothing playing, there's nothing for them to come after. -->
      <li v-if="currentPlayable" data-testid="queue-after-current" tabindex="0" @click="queueAfterCurrent">
        Play next
      </li>
      <li data-testid="queue-bottom" tabindex="0" @click="queueToBottom">Add to queue</li>
    </ul>

    <section class="existing-playlists">
      <p class="mb-2 text-[0.9rem]">Add {{ pluralize(playables, 'song') }} to</p>

      <ul class="scroll-mask-y relative max-h-48 overflow-y-scroll space-y-1.5" @keydown.enter.space.prevent="choose">
        <li
          v-if="config.favorites"
          class="favorites"
          data-testid="add-to-favorites"
          tabindex="0"
          @click="addToFavorites"
        >
          Favorites
        </li>

        <li
          v-for="playlist in playlists"
          :key="playlist.id"
          class="playlist"
          data-testid="add-to-playlist"
          tabindex="0"
          @click="addToExistingPlaylist(playlist)"
        >
          {{ playlist.name }}
        </li>
      </ul>
    </section>

    <M3Button variant="outlined" class="w-full!" @click.prevent="addToNewPlaylist"> New playlist… </M3Button>
  </div>
</template>

<script lang="ts" setup>
import { computed, toRef, toRefs, watch } from 'vue'
import { pluralize } from '@/utils/formatters'
import { usePlaylistStore } from '@/stores/playlistStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableMenuMethods } from '@/composables/usePlayableMenuMethods'

import M3Button from '@/components/m3/M3Button.vue'

const playlistStore = usePlaylistStore()
const queueStore = useQueueStore()

const props = defineProps<{ playables: Playable[]; config: AddToMenuConfig }>()
const emit = defineEmits<{ (e: 'closing'): void }>()

const { playables, config } = toRefs(props)

const currentPlayable = computed(() => queueStore.current)

const allPlaylists = toRef(playlistStore.state, 'playlists')
const playlists = computed(() => allPlaylists.value.filter(({ is_smart }) => !is_smart))

const close = () => emit('closing')

const { queueAfterCurrent, queueToBottom, addToFavorites, addToExistingPlaylist, addToNewPlaylist } =
  usePlayableMenuMethods(playables, close)

/** Enter or Space chooses an item, as a click does. */
const choose = (event: KeyboardEvent) => (event.target as HTMLElement).click()

watch(playables, () => playables.value.length || close())
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
li {
  @apply h-9 leading-9 py-0 px-3 truncate rounded-sm bg-(--schemes-surface-container-high) cursor-pointer
  hover:bg-(--schemes-primary) hover:text-(--schemes-on-primary);
}
</style>
