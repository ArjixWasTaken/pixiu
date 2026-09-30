<template>
  <M3SegmentedButton v-model="value" :segments data-testid="view-mode-switch" />
</template>

<script lang="ts" setup>
import { computed } from 'vue'

import M3SegmentedButton from '@/components/m3/M3SegmentedButton.vue'

const props = withDefaults(defineProps<{ secondary?: 'list' | 'table' }>(), { secondary: 'list' })

const value = defineModel<ViewMode>({ default: 'grid' })

const segments = computed(() => [
  { id: 'grid', label: 'Grid' },
  props.secondary === 'table' ? { id: 'table', label: 'Table' } : { id: 'list', label: 'List' },
])
</script>
