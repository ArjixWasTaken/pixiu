<template>
  <article class="flex flex-col items-center min-w-[24px]">
    <input
      :id
      ref="input"
      v-model.number="value"
      :max="MAX"
      :min="MIN"
      class="slider"
      step="0.1"
      type="range"
      @change="emit('commit')"
    />
    <label :for="id" class="mt-2 mb-0 text-left text-sm text-(--schemes-on-surface)">
      <slot />
    </label>
  </article>
</template>

<script lang="ts" setup>
import { computed, useId, useTemplateRef } from 'vue'

const props = withDefaults(
  defineProps<{
    type?: 'preamp' | 'gain'
    modelValue?: number
  }>(),
  {
    type: 'gain',
    modelValue: 0,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
  (e: 'commit'): void
}>()

const MIN = -20
const MAX = 20
/** The handle's diameter, as styled below. */
const HANDLE = 13

const id = useId()
const input = useTemplateRef('input')

const value = computed({
  get: () => props.modelValue,
  set: value => emit('update:modelValue', value),
})

/** Sets the band from outside, as when a preset loads. */
const updateSliderValue = (val: number) => (value.value = val)

/** Where the handle's center is on the page, for the curve drawn through the bands. */
const handleCenter = () => {
  const rect = input.value!.getBoundingClientRect()
  const fraction = (value.value - MIN) / (MAX - MIN)

  return {
    x: rect.left + rect.width / 2,
    y: rect.top + HANDLE / 2 + (1 - fraction) * (rect.height - HANDLE),
  }
}

defineExpose({ updateSliderValue, handleCenter })
</script>

<style scoped>
/* Vertical, top is +20: a thin line with a round handle, the curve drawn through the handles. */
.slider {
  writing-mode: vertical-lr;
  direction: rtl;
  appearance: none;
  width: 16px;
  height: 100px;
  margin: 0;
  background: linear-gradient(
      to bottom,
      transparent,
      color-mix(in srgb, var(--schemes-on-surface) 15%, transparent) 15%,
      color-mix(in srgb, var(--schemes-on-surface) 15%, transparent) 85%,
      transparent
    )
    center / 1px 100% no-repeat;
  cursor: ns-resize;

  &::-webkit-slider-runnable-track {
    background: transparent;
  }

  &::-moz-range-track {
    background: transparent;
  }

  &::-webkit-slider-thumb {
    appearance: none;
    width: 13px;
    height: 13px;
    border: 0;
    border-radius: 9999px;
    background: var(--schemes-primary);
    cursor: pointer;
  }

  &::-moz-range-thumb {
    width: 13px;
    height: 13px;
    border: 0;
    border-radius: 9999px;
    background: var(--schemes-primary);
    cursor: pointer;
  }

  &:focus-visible {
    outline: 2px solid var(--schemes-secondary);
    outline-offset: 2px;
    border-radius: 8px;
  }
}
</style>
