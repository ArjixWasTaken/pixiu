<template>
  <span v-if="standing === 'hoarded'" class="standing hoarded"> <Icon :icon="faCheck" /> In your library </span>
  <span v-else-if="standing === 'pending'" class="standing pending"> <Icon :icon="faSpinner" spin /> On its way </span>
  <Btn v-else size="small" @click.prevent="emit('grab')">Download</Btn>
</template>

<script lang="ts" setup>
import { faCheck, faSpinner } from '@fortawesome/free-solid-svg-icons'
import type { Standing } from '@/services/huntingService'

import Btn from '@/components/ui/form/Btn.vue'

defineProps<{ standing: Standing }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.standing {
  @apply inline-flex items-center gap-2 text-sm;
}

.hoarded {
  @apply text-k-success;
}

.pending {
  @apply text-k-fg-70;
}
</style>
