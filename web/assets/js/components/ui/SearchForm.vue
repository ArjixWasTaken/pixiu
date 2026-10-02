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
import { onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue'
import { useDebounceFn } from '@vueuse/core'
import { eventBus } from '@/utils/eventBus'
import { useRouter } from '@/composables/useRouter'

import M3SearchBar from '@/components/m3/M3SearchBar.vue'

const props = withDefaults(defineProps<{ autofocus?: boolean }>(), { autofocus: false })
const emit = defineEmits<{ (e: 'focus-change', focused: boolean): void }>()

const { go, replace, url, isCurrentScreen, getRouteParam, onRouteChanged } = useRouter()

const placeholder = 'Search'

const bar = useTemplateRef('bar')

/** What the search screens are looking for: the `q` of their URL. */
const searched = () => (isCurrentScreen('Search.Excerpt', 'Search.Playables') ? getRouteParam('q') || '' : '')

const q = ref(searched())

const resultsFor = (words: string) => (words ? `${url('search')}?q=${encodeURIComponent(words)}` : url('search'))

/**
 * The words go in the search screen's URL, so the results and the field agree,
 * and Back and reloads bring both back. While on the results, the URL is
 * replaced word by word; from elsewhere, the results open as a new page.
 */
const search = () => {
  const words = q.value.trim()

  if (isCurrentScreen('Search.Excerpt')) {
    words !== searched() && replace(resultsFor(words))
  } else if (words) {
    go(resultsFor(words))
  }
}

const debouncedSearch = window.RUNNING_UNIT_TESTS ? search : useDebounceFn(search, 400)

const onInput = () => debouncedSearch()

const onSubmit = () => {
  const words = q.value.trim()
  words && go(resultsFor(words))
}

// Focusing the field is not searching yet: Tab passes through it without leaving the page.
const onFocus = () => emit('focus-change', true)

const onBlur = () => emit('focus-change', false)

const focus = () => bar.value?.focus()

onMounted(() => {
  eventBus.on('FOCUS_SEARCH_FIELD', focus)
  props.autofocus && focus()
})

onBeforeUnmount(() => eventBus.off('FOCUS_SEARCH_FIELD', focus))

// Leaving the results leaves the search behind; coming back to them (Back,
// a link) brings their words back. Words still being typed stay as they are.
onRouteChanged(() => {
  const words = searched()
  words !== q.value.trim() && (q.value = words)
})
</script>
