<template>
  <div :class="{ disabled }" :style="{ '--fraction': fraction }" class="m3-slider">
    <span class="track active" />
    <span class="handle" />
    <span class="track inactive" />
    <input
      v-model.number="value"
      :aria-label="label"
      :disabled
      :max
      :min
      :step
      :aria-valuetext="valueText"
      type="range"
      @change="emit('commit', value)"
      @keydown="onKeydown"
    />
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'

/**
 * The Material 3 slider: a thick track split around a bar-shaped handle. A
 * transparent native range input on top does the work, so keyboard and
 * screen readers behave as usual.
 */
const props = withDefaults(
  defineProps<{
    min?: number
    max?: number
    step?: number | 'any'
    /** How far an arrow key moves the handle; `step` when unset. Dragging keeps `step`. */
    keyStep?: number
    label?: string
    /** What a screen reader says for the value ("1:05 of 3:18"). */
    valueText?: string
    disabled?: boolean
  }>(),
  {
    min: 0,
    max: 100,
    step: 'any',
    keyStep: undefined,
    valueText: undefined,
    disabled: false,
  },
)

const emit = defineEmits<{ (e: 'commit', value: number): void }>()

const value = defineModel<number>({ default: 0 })

/** Arrow keys move by `keyStep`, where the native step would be too fine (seeking by 0.1 s). */
const onKeydown = (event: KeyboardEvent) => {
  const by = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 }[event.key]

  if (!props.keyStep || !by || props.disabled) {
    return
  }

  event.preventDefault()
  value.value = Math.min(props.max, Math.max(props.min, value.value + by * props.keyStep))
  emit('commit', value.value)
}

const fraction = computed(() => {
  const span = props.max - props.min
  return span > 0 ? Math.min(1, Math.max(0, (value.value - props.min) / span)) : 0
})
</script>

<style scoped>
.m3-slider {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  height: var(--m3-slider-height);
  min-width: 48px;
  flex: 1;
}

.track {
  height: var(--m3-slider-track);
  min-width: 0;
}

.active {
  flex: 0 0 calc((100% - 16px) * var(--fraction));
  border-radius: calc(var(--m3-slider-track) / 2) 2px 2px calc(var(--m3-slider-track) / 2);
  background: var(--schemes-primary);
}

.inactive {
  flex: 1;
  border-radius: 2px calc(var(--m3-slider-track) / 2) calc(var(--m3-slider-track) / 2) 2px;
  background: var(--schemes-secondary-container);
}

.handle {
  width: 4px;
  height: var(--m3-slider-height);
  flex-shrink: 0;
  border-radius: 2px;
  background: var(--schemes-primary);
  transition: width 100ms linear;
}

input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: pointer;
}

.m3-slider:has(input:active) .handle,
.m3-slider:has(input:focus-visible) .handle {
  width: 2px;
}

.m3-slider:has(input:focus-visible) {
  outline: 2px solid var(--schemes-secondary);
  outline-offset: 2px;
  border-radius: 8px;
}

.disabled {
  opacity: 0.38;

  input {
    cursor: default;
  }
}
</style>
