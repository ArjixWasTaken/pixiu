<template>
  <div class="relative" data-testid="song-list-controls">
    <div class="flex gap-2 flex-wrap items-center">
      <template v-if="altPressed">
        <M3Button
          v-if="selectedPlayables.length < 2 && filteredPlayables.length"
          class="btn-play-all"
          icon="play_arrow"
          title="Play all. Press Alt/⌥ to change mode."
          @click.prevent="playAll"
        >
          Play all
        </M3Button>

        <M3Button
          v-if="selectedPlayables.length > 1"
          class="btn-play-selected"
          icon="play_arrow"
          title="Play selected. Press Alt/⌥ to change mode."
          @click.prevent="playSelected"
        >
          Play selected
        </M3Button>
      </template>

      <template v-else>
        <M3Button
          v-if="selectedPlayables.length < 2 && filteredPlayables.length"
          class="btn-shuffle-all"
          data-testid="btn-shuffle-all"
          icon="shuffle"
          title="Shuffle all. Press Alt/⌥ to change mode."
          @click.prevent="shuffle"
        >
          Shuffle all
        </M3Button>

        <M3Button
          v-if="selectedPlayables.length > 1"
          class="btn-shuffle-selected"
          data-testid="btn-shuffle-selected"
          icon="shuffle"
          title="Shuffle selected. Press Alt/⌥ to change mode."
          @click.prevent="shuffleSelected"
        >
          Shuffle selected
        </M3Button>
      </template>

      <span v-if="showAddToButton" ref="addToButton" class="inline-flex">
        <M3Button :icon="showingAddToMenu ? 'close' : 'playlist_add'" variant="tonal">
          {{ showingAddToMenu ? 'Cancel' : 'Add to…' }}
        </M3Button>
      </span>

      <M3Button
        v-if="config.clearQueue"
        icon="clear_all"
        title="Clear current queue"
        variant="outlined"
        @click.prevent="clearQueue"
      >
        Clear queue
      </M3Button>

      <M3IconButton v-if="config.refresh" icon="refresh" label="Refresh" @click.prevent="refresh" />

      <slot />

      <ListFilter v-if="config.filter && allPlayables.length" class="ml-2" />
    </div>

    <Popover
      v-if="showAddToButton"
      ref="popover"
      :anchor="addToButton"
      class="context-menu p-0"
      @toggle="showingAddToMenu = $event"
    >
      <AddToMenu :config="config.addTo" :playables="selectedPlayables" @closing="popover?.hide()" />
    </Popover>
  </div>
</template>

<script lang="ts" setup>
import type { Ref } from 'vue'
import { computed, onBeforeUnmount, onMounted, ref, toRef, watch } from 'vue'
import { FilteredPlayablesKey, PlayablesKey, SelectedPlayablesKey } from '@/config/symbols'
import { requireInjection } from '@/utils/helpers'

import AddToMenu from '@/components/playable/AddToMenu.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import ListFilter from '@/components/ui/ListFilter.vue'
import Popover from '@/components/ui/Popover.vue'

const props = defineProps<{ config: PlayableListControlsConfig }>()

const emit = defineEmits<{
  (e: 'play-all' | 'play-selected', shuffle: boolean): void
  (e: 'filter', keywords: string): void
  (e: 'clear-queue' | 'delete-playlist' | 'refresh'): void
}>()

const config = toRef(props, 'config')

const [allPlayables] = requireInjection<[Ref<Playable[]>]>(PlayablesKey)
const [filteredPlayables] = requireInjection<[Ref<Playable[]>]>(FilteredPlayablesKey)
const [selectedPlayables] = requireInjection<[Ref<Playable[]>]>(SelectedPlayablesKey)

const addToButton = ref<HTMLElement>()
const popover = ref<InstanceType<typeof Popover>>()
const showingAddToMenu = ref(false)
const altPressed = ref(false)

const showAddToButton = computed(() => Boolean(selectedPlayables.value.length))

// When the AddTo trigger button disappears (no items selected), the Popover
// is unmounted via v-if without firing @toggle(false), so we reset the menu
// open-state flag explicitly. Otherwise the trigger's "Cancel" / "Add To…"
// label could be stuck on "Cancel" if items are reselected later.
watch(showAddToButton, visible => {
  if (!visible) {
    showingAddToMenu.value = false
  }
})

const shuffle = () => emit('play-all', true)
const shuffleSelected = () => emit('play-selected', true)
const playAll = () => emit('play-all', false)
const playSelected = () => emit('play-selected', false)
const clearQueue = () => emit('clear-queue')
const refresh = () => emit('refresh')
const registerKeydown = (event: KeyboardEvent) => event.key === 'Alt' && (altPressed.value = true)
const registerKeyup = (event: KeyboardEvent) => event.key === 'Alt' && (altPressed.value = false)

onMounted(() => {
  window.addEventListener('keydown', registerKeydown)
  window.addEventListener('keyup', registerKeyup)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', registerKeydown)
  window.removeEventListener('keyup', registerKeyup)
})
</script>
