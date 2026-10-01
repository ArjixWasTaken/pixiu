<template>
  <div
    v-koel-focus
    class="reorder-blocks-modal flex flex-col"
    data-testid="reorder-blocks-modal"
    tabindex="0"
    @keydown.esc="close"
  >
    <header>
      <h1>Home blocks</h1>
    </header>

    <main class="space-y-1">
      <p class="hint m3-body-medium">
        Drag to reorder; untick to hide. Blocks with nothing to show stay hidden until they have something.
      </p>
      <div
        v-for="block in orderedBlocks"
        :key="block.id"
        :draggable="true"
        class="group flex transition-all items-center gap-2 pr-3 py-2 rounded-sm bg-k-bg-secondary hover:bg-k-fg-5 hover:pl-3 cursor-grab active:cursor-grabbing active:text-k-highlight select-none"
        :class="{ 'opacity-40': draggedId === block.id }"
        @dragstart="onDragStart(block, $event)"
        @dragover.prevent="onDragOver(block, $event)"
        @dragend="onDragEnd"
        @drop.prevent
      >
        <GripVerticalIcon class="w-4 h-4 text-k-fg-50" />
        <span class="flex-1">{{ block.label }}</span>
        <M3Checkbox
          :aria-label="`Show ${block.label}`"
          :model-value="!hidden.includes(block.id)"
          @update:model-value="setShown(block.id, $event)"
        />
      </div>
    </main>

    <footer>
      <Btn @click.prevent="close">Close</Btn>
    </footer>
  </div>
</template>

<script lang="ts" setup>
import { isEqual } from 'lodash-es'
import { GripVerticalIcon } from 'lucide-vue-next'
import { computed, ref } from 'vue'
import { preferenceStore } from '@/stores/preferenceStore'

import Btn from '@/components/ui/form/Btn.vue'
import M3Checkbox from '@/components/m3/M3Checkbox.vue'

interface BlockSummary {
  id: string
  label: string
}

const props = defineProps<{ blocks: BlockSummary[] }>()

const emit = defineEmits<{ (e: 'close'): void }>()

const draggedId = ref<string | null>(null)
// `blocks` arrives already sorted by the parent (HomeScreen passes
// `orderedBlocks`), so we just adopt that order as the initial state.
const orderIds = ref<string[]>(props.blocks.map(block => block.id))

const orderedBlocks = computed(() => orderIds.value.map(id => props.blocks.find(block => block.id === id)!))

const onDragStart = (block: BlockSummary, event: DragEvent) => {
  if (!event.dataTransfer) {
    return
  }

  event.dataTransfer.effectAllowed = 'move'
  draggedId.value = block.id
}

const onDragOver = (target: BlockSummary, event: DragEvent) => {
  if (draggedId.value === null || draggedId.value === target.id) {
    return
  }

  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  const insertBefore = event.clientY < rect.top + rect.height / 2

  const next = orderIds.value.filter(id => id !== draggedId.value)
  const targetIndex = next.indexOf(target.id)

  if (targetIndex === -1) {
    return
  }

  next.splice(insertBefore ? targetIndex : targetIndex + 1, 0, draggedId.value)

  if (!isEqual(next, orderIds.value)) {
    orderIds.value = next
  }
}

const onDragEnd = () => {
  if (draggedId.value === null) {
    return
  }

  if (!isEqual(orderIds.value, preferenceStore.home_blocks_order ?? [])) {
    preferenceStore.home_blocks_order = [...orderIds.value]
  }

  draggedId.value = null
}

const hidden = computed(() => preferenceStore.home_blocks_hidden ?? [])

const setShown = (id: string, shown: boolean) => {
  const others = hidden.value.filter(other => other !== id)
  preferenceStore.home_blocks_hidden = shown ? others : [...others, id]
}

const close = () => emit('close')
</script>

<style scoped>
.hint {
  margin-bottom: 12px;
  color: var(--schemes-on-surface-variant);
}
</style>
