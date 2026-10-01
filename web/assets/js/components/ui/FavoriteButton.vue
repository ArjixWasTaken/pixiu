<template>
  <M3IconButton
    :class="{ favorite }"
    :icon-size="size === 'md' ? 24 : 20"
    :label="title"
    :selected="favorite"
    class="favorite-button"
    icon="favorite"
    @click.prevent.stop="emit('toggle')"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'

import M3IconButton from '@/components/m3/M3IconButton.vue'

const props = withDefaults(defineProps<{ favorite: boolean; size?: 'sm' | 'md' }>(), { size: 'sm' })
const emit = defineEmits<{ (e: 'toggle'): void }>()

const title = computed(() => (props.favorite ? 'Remove from favorites' : 'Add to favorites'))
</script>

<style scoped>
.favorite {
  color: var(--schemes-primary);
}
</style>
