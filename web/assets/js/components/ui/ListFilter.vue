<template>
  <OnClickOutside @trigger="maybeHideInput">
    <M3Chip v-if="!showingInput" icon="filter_list" title="Filter" @click.prevent="showInput">Filter</M3Chip>
    <form v-else class="filter-field m3-label-large" @submit.prevent>
      <M3Icon :size="18" class="text-(--schemes-primary)" name="filter_list" />
      <input
        ref="input"
        v-model="keywords"
        aria-label="Filter"
        placeholder="Keywords"
        type="search"
        @blur="inputting = false"
        @focus="inputting = true"
        @keydown.esc="clear"
      />
      <button v-if="keywords" aria-label="Clear the filter" class="clear" type="button" @click="clear">
        <M3Icon :size="18" name="close" />
      </button>
    </form>
  </OnClickOutside>
</template>

<script lang="ts" setup>
import { OnClickOutside } from '@vueuse/components'
import { computed, nextTick, ref } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { FilterKeywordsKey } from '@/config/symbols'

import M3Chip from '@/components/m3/M3Chip.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const input = ref<HTMLInputElement>()
const inputting = ref(false)

const keywords = requireInjection(FilterKeywordsKey, ref(''))

// We show the input if the user is currently typing in it, or if there are any keywords entered
const showingInput = computed(() => inputting.value || keywords.value.trim())

const maybeHideInput = () => {
  inputting.value = false
}

/** Shows everything again. */
const clear = () => {
  keywords.value = ''
  inputting.value = false
}

const showInput = () => {
  inputting.value = true

  nextTick(() => {
    input.value?.focus()
    input.value?.select()
  })
}
</script>

<style scoped>
.filter-field {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 12px 0 8px;
  border-radius: 8px;
  border: 1px solid var(--schemes-outline);
  color: var(--schemes-on-surface);

  input {
    width: 160px;
    background: transparent;
    border: 0;
    outline: 0;
    color: inherit;

    &::-webkit-search-cancel-button {
      display: none;
    }
  }

  .clear {
    display: flex;
    margin-right: -4px;
    color: var(--schemes-on-surface-variant);
  }
}
</style>
