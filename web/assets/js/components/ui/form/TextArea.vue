<template>
  <textarea ref="el" v-model="value" :aria-label="ariaLabel" class="k-field w-full h-48 py-3" />
</template>

<script lang="ts" setup>
import { computed, ref, useAttrs } from 'vue'

const value = defineModel<string>()
const attrs = useAttrs()

// Aria label is optional, but if it's not provided, we'll use the name prop as a fallback.
// This is useful for both accessibility and testing, since getByRole('textbox', { name: '...' })
// looks for aria-label.
const ariaLabel = computed(() => String(attrs.ariaLabel || attrs.name))

const el = ref<HTMLTextAreaElement>()

defineExpose({
  el,
})
</script>
