<template>
  <div
    ref="wrapper"
    class="playable-list-wrap relative flex flex-col flex-1 py-0"
    data-testid="song-list"
    @keydown="onKeydown"
  >
    <PlayableListHeader v-if="config.hasHeader" @sort="sort" />

    <VirtualScroller
      ref="virtualScroller"
      v-slot="{ item }: { item: PlayableRow }"
      :item-height="songItemHeight"
      :items="rows"
      @scrolled-to-end="$emit('scrolled-to-end')"
    >
      <PlayableListItem
        :key="item.playable.id"
        :item="item"
        :show-disc="showDiscLabel(item.playable)"
        :draggable="!isTouch"
        @click="onClick(item, $event)"
        @dragleave="onDragLeave"
        @dragstart="onDragStart(item, $event)"
        @play="onPlay(item.playable)"
        @contextmenu.prevent="isTouch || onContextMenu(item, $event)"
        @request-context-menu="onContextMenu(item, $event)"
        @dragover.prevent="onDragOver"
        @drop.prevent="onDrop(item, $event)"
        @dragend.prevent="onDragEnd"
      />
    </VirtualScroller>
  </div>
</template>

<script lang="ts" setup>
import { useEventListener, useSwipe, useThrottleFn } from '@vueuse/core'
import { useViewport } from '@/composables/useViewport'
import type { Ref } from 'vue'
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { defineAsyncComponent, requireInjection } from '@/utils/helpers'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useQueueStore } from '@/stores/queueStore'
import { useDraggable, useDroppable } from '@/composables/useDragAndDrop'
import { useListSelection } from '@/composables/useListSelection'
import { playback } from '@/services/playbackManager'
import { useContextMenu } from '@/composables/useContextMenu'
import { useSizeVariable } from '@/composables/useSizeVariable'

import {
  FilteredPlayablesKey,
  PlayableListConfigKey,
  PlayableListContextKey,
  PlayableListSortFieldKey,
  SelectedPlayablesKey,
} from '@/config/symbols'

import PlayableListItem from '@/components/playable/playable-list/PlayableListItem.vue'
import VirtualScroller from '@/components/ui/VirtualScroller.vue'
import PlayableListHeader from '@/components/playable/playable-list/PlayableListHeader.vue'

const preferences = usePreferenceStore()
const queueStore = useQueueStore()

const { isTouch } = useViewport()

const emit = defineEmits<{
  (e: 'press:enter', event: KeyboardEvent): void
  (e: 'press:delete'): void
  (e: 'reorder', song: Playable, placement: Placement): void
  (e: 'sort', field: MaybeArray<PlayableListSortField>, order: SortOrder): void
  (e: 'swipe', direction: 'up' | 'down'): void
  (e: 'scrolled-to-end'): void
}>()

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))

const { startDragging } = useDraggable('playables')
const { getDroppedData, acceptsDrop } = useDroppable(['playables'])
const { openContextMenu } = useContextMenu()

const [playables] = requireInjection<[Ref<Playable[]>]>(FilteredPlayablesKey)
const [selectedPlayables, setSelectedPlayables] = requireInjection<[Ref<Playable[]>, Closure]>(SelectedPlayablesKey)
const [sortField] = requireInjection<[Ref<MaybeArray<PlayableListSortField>>, Closure]>(PlayableListSortFieldKey)
const [config] = requireInjection<[Partial<PlayableListConfig>]>(PlayableListConfigKey, [{}])
const [context] = requireInjection<[PlayableListContext]>(PlayableListContextKey)

const wrapper = ref<HTMLElement>()
const virtualScroller = ref<InstanceType<typeof VirtualScroller>>()
const sortFields = ref<PlayableListSortField[]>([])

// A swipe or a wheel turn up or down, for the screen to fold its header.
useSwipe(wrapper, {
  threshold: 30,
  onSwipeEnd: (_, direction) => (direction === 'up' || direction === 'down') && emit('swipe', direction),
})

useEventListener(
  wrapper,
  'wheel',
  useThrottleFn(
    (event: WheelEvent) => Math.abs(event.deltaY) >= 5 && emit('swipe', event.deltaY > 0 ? 'down' : 'up'),
    50,
  ),
  { passive: true },
)

const rows = computed(() => {
  return playables.value.map<PlayableRow>(playable => {
    return reactive({
      playable,
      selected: false,
    })
  })
})

const {
  select,
  selectAllWithKeyboard,
  clearSelection,
  toggleSelected,
  isSelected,
  selectBetween,
  inSelectedRange,
  lastSelected,
  selected,
  reapplySelection,
} = useListSelection(rows, 'playable.id')

const shouldTriggerContinuousPlayback = computed(() => {
  return (
    preferences.continuous_playback &&
    typeof context.type !== 'undefined' &&
    ['Playlist', 'Album', 'Artist', 'Genre', 'Favorites'].includes(context.type)
  )
})

const getAllPlayablesWithSort = () => rows.value.map(row => row.playable)

watch(selected, () => setSelectedPlayables(selected.value.map(({ playable }) => playable)), { deep: true })

const sort = (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  // we simply pass the sort event from the header up to the parent component
  emit('sort', field, order)
}

const render = () => {
  config.sortable || (sortFields.value = [])
  reapplySelection()
}

watch(playables, () => render(), { deep: true })

const handleDelete = () => {
  emit('press:delete') // eslint-disable-line vue/custom-event-name-casing
  clearSelection()
}

const handleEnter = (event: KeyboardEvent) => {
  emit('press:enter', event) // eslint-disable-line vue/custom-event-name-casing
  clearSelection()
}

/**
 * Enter, Delete and Ctrl/Cmd+A act on the songs, when a song has focus: a
 * field or button in the list (the filter, the sort menu, a row's buttons)
 * keeps its keys ("a" types an "a", Enter opens a menu).
 */
const onKeydown = (event: KeyboardEvent) => {
  if (!(event.target instanceof Element) || !event.target.matches('.song-item')) {
    return
  }

  if (event.key === 'Enter') {
    event.preventDefault()
    event.stopPropagation()
    handleEnter(event)
  } else if (event.key === 'Delete' || event.key === 'Backspace') {
    event.preventDefault()
    event.stopPropagation()
    handleDelete()
  } else if (event.key.toLowerCase() === 'a' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault()
    selectAllWithKeyboard(event)
  }
}

const onDragStart = async (row: PlayableRow, event: DragEvent) => {
  // If the user is dragging an unselected row, clear the current selection.
  if (!isSelected(row)) {
    clearSelection()
    select(row)
    await nextTick()
  }

  // Add "dragging" class to the wrapper so that we can disable pointer events on child elements.
  // This prevents dragleave events from firing when the user drags the mouse over the child elements.
  wrapper.value?.classList.add('dragging')

  startDragging(event, selectedPlayables.value)
}

let currentDropTarget: HTMLElement | null = null

const clearDropTarget = () => {
  currentDropTarget?.classList.remove('droppable', 'dragover-top', 'dragover-bottom')
  currentDropTarget = null
}

const onDragOver = useThrottleFn((event: DragEvent) => {
  if (!config.reorderable) {
    return
  }

  if (acceptsDrop(event)) {
    const target = (event.target as HTMLElement).closest<HTMLElement>('.song-item')

    if (!target) {
      return
    }

    // If we moved to a different item, clear the old one
    if (currentDropTarget && currentDropTarget !== target) {
      clearDropTarget()
    }

    currentDropTarget = target

    const rect = target.getBoundingClientRect()
    const midPoint = rect.top + rect.height / 2
    target.classList.remove('dragover-top', 'dragover-bottom')
    target.classList.add('droppable', event.clientY < midPoint ? 'dragover-top' : 'dragover-bottom')
  }

  return false
}, 50)

const onDragLeave = (event: DragEvent) => {
  // Only clear if the cursor actually left the item (not just moved between children)
  const related = event.relatedTarget

  if (!(related instanceof Node) || !currentDropTarget?.contains(related)) {
    clearDropTarget()
  }

  return false
}

const onDrop = (row: PlayableRow, event: DragEvent) => {
  if (!config.reorderable || !getDroppedData(event) || !selectedPlayables.value.length) {
    wrapper.value?.classList.remove('dragging')
    clearDropTarget()
    return false
  }

  wrapper.value?.classList.remove('dragging')

  if (!inSelectedRange(row)) {
    emit('reorder', row.playable, currentDropTarget?.classList.contains('dragover-bottom') ? 'after' : 'before')
  }

  clearDropTarget()
  return false
}

const onDragEnd = () => {
  wrapper.value?.classList.remove('dragging')
  clearDropTarget()
}

const onClick = (row: PlayableRow, event: MouseEvent) => {
  // A finger plays what it taps; its long press (or ⋮) opens the song's menu.
  if (isTouch.value) {
    onPlay(row.playable)
    return
  }

  if (event.ctrlKey || event.metaKey) {
    toggleSelected(row)
  }

  if (event.button === 0) {
    if (!(event.ctrlKey || event.metaKey || event.shiftKey)) {
      clearSelection()
      toggleSelected(row)
    }

    if (event.shiftKey && lastSelected.value) {
      selectBetween(lastSelected.value, row)
    }
  }
}

const onContextMenu = async (row: PlayableRow, event: MouseEvent) => {
  // On a phone, the menu is for the song it was opened on; nothing gets selected.
  if (isTouch.value) {
    openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [row.playable] })
    return
  }

  if (!isSelected(row)) {
    clearSelection()
    toggleSelected(row)

    // await a tick so that the selected items are collected properly
    await nextTick()
  }

  openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, {
    playables: selectedPlayables.value,
  })
}

const onPlay = async (playable: Playable) => {
  if (playable.playback_state === 'Stopped') {
    if (shouldTriggerContinuousPlayback.value) {
      queueStore.replaceQueueWith(getAllPlayablesWithSort())
    }

    await playback().play(playable)
  } else if (playable.playback_state === 'Paused') {
    await playback().resume()
  } else {
    await playback().pause()
  }
}

const discIndexMap = computed(() => {
  const map: { [key: number]: number } = {}

  rows.value.forEach((row, index) => {
    const { disc } = row.playable as Song
    if (!Object.values(map).includes(disc)) {
      map[index] = disc
    }
  })

  return map
})

const noOrOneDiscOnly = computed(() => Object.keys(discIndexMap.value).length <= 1)
const sortingByTrack = computed(() => sortField.value === 'track')
const inAlbumContext = computed(() => context.type === 'Album')

const noDiscLabel = computed(() => noOrOneDiscOnly.value || !sortingByTrack.value || !inAlbumContext.value)

const showDiscLabel = (row: Playable) => {
  if (noDiscLabel.value) {
    return false
  }

  const index = rows.value.findIndex(({ playable }) => playable.id === row.id)
  return discIndexMap.value[index] !== undefined
}

/** The height of a row, as an estimate: the scroller measures each (those with a disc label are taller). */
const songItemHeight = useSizeVariable('--m3-row-height', 72)

const scrollToPlayable = (playable: Playable) => {
  const index = rows.value.findIndex(row => row.playable.id === playable.id)

  if (index >= 0) {
    virtualScroller.value?.scrollToIndex(index)
  }
}

defineExpose({
  getAllPlayablesWithSort,
  scrollToPlayable,
})

onMounted(() => render())
</script>

<style lang="postcss">
.playable-list-wrap {
  .virtual-scroller {
    flex: 1;
  }

  &.dragging .song-item * {
    pointer-events: none;
  }
}
</style>
