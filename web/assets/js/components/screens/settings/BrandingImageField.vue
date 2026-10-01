<template>
  <fieldset class="flex items-start gap-4 py-3">
    <img
      :src="model"
      alt=""
      class="size-[100px] shrink-0 rounded-md bg-(--schemes-surface-container-high) object-contain p-1"
    />

    <div class="min-w-0 flex-1">
      <h4 :id="`${fieldId}-label`" class="text-(--schemes-on-surface)">
        <slot name="label" />
      </h4>
      <p class="text-[.95rem] text-(--schemes-on-surface-variant) text-pretty">
        <slot name="help" />
      </p>
      <p class="text-[.95rem] text-(--schemes-on-surface-variant)">Recommended size: 512×512 pixels or larger.</p>

      <div class="mt-3 flex items-center gap-4">
        <label
          class="relative inline-flex cursor-pointer items-center rounded-md border border-(--schemes-outline-variant) px-3 py-1.5 hover:bg-(--schemes-surface-container-highest) has-focus-visible:outline-2 has-focus-visible:outline-(--schemes-primary)"
        >
          <input
            :aria-labelledby="`${fieldId}-change ${fieldId}-label`"
            :name
            accept="image/*"
            class="sr-only"
            type="file"
            @change="onImageInputChange"
          />
          <span :id="`${fieldId}-change`">Change</span>
        </label>
        <button
          v-if="hasCustomValue"
          class="text-(--schemes-on-surface-variant) hover:text-(--schemes-on-surface)"
          type="button"
          @click.prevent="removeCustomValue"
        >
          Reset
        </button>
      </div>
    </div>
  </fieldset>
</template>

<script setup lang="ts">
import { computed, onMounted, useId } from 'vue'
import { useImageFileInput } from '@/composables/useImageFileInput'

const props = defineProps<{ default: string; name: string }>()

const model = defineModel<string>()
const fieldId = useId()
let initialValue: typeof model.value

const hasCustomValue = computed(() => model.value && model.value !== props.default)

const removeCustomValue = () => {
  // First reset the model to the initial value (current settings), then to the default fallback.
  model.value = model.value === initialValue ? props.default : initialValue
}

const { onImageInputChange } = useImageFileInput({
  onImageDataUrl: dataUrl => (model.value = dataUrl),
})

onMounted(() => (initialValue = model.value))
</script>
