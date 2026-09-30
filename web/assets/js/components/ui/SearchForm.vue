<template>
  <M3SearchBar
    id="searchForm"
    ref="bar"
    v-model="q"
    :placeholder
    autocorrect="off"
    name="q"
    spellcheck="false"
    @blur="onBlur"
    @focus="onFocus"
    @input="onInput"
    @submit="onSubmit"
  >
    <template v-if="$slots.trailing" #trailing>
      <slot name="trailing" />
    </template>
  </M3SearchBar>
</template>

<script lang="ts" setup>
import { computed, ref, useTemplateRef } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'
import { useViewport } from '@/composables/useViewport'

import M3SearchBar from '@/components/m3/M3SearchBar.vue'

const emit = defineEmits<{ (e: 'focus-change', focused: boolean): void }>()

const { go, url } = useRouter()
const { isMobile } = useViewport()

const placeholder = computed(() => (isMobile.value ? 'Search' : 'Search songs, artists and albums'))

const bar = useTemplateRef('bar')
const q = ref('')

let onInput = () => {
  const trimmed = q.value.trim()
  trimmed && eventBus.emit('SEARCH_KEYWORDS_CHANGED', trimmed)
}

if (!window.RUNNING_UNIT_TESTS) {
  onInput = useDebounceFn(onInput, 500)
}

const onSubmit = () => go(url('search'))

const onFocus = () => {
  emit('focus-change', true)
  isMobile.value || go(url('search'))
}

const onBlur = () => emit('focus-change', false)

eventBus.on('FOCUS_SEARCH_FIELD', () => bar.value?.focus())
</script>
