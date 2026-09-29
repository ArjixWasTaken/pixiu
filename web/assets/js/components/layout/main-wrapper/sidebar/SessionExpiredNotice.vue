<template>
  <a v-if="expired" :href="url('settings')" class="notice">
    <Icon :icon="faTriangleExclamation" />
    <span>YouTube Music signed píxiū out. Log in again.</span>
  </a>
</template>

<script lang="ts" setup>
import { faTriangleExclamation } from '@fortawesome/free-solid-svg-icons'
import { computed } from 'vue'
import { huntingStore } from '@/stores/huntingStore'
import { useRouter } from '@/composables/useRouter'

const { url } = useRouter()

const expired = computed(() => huntingStore.state.session === 'expired')
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.notice {
  @apply flex items-start gap-2 mx-6 mb-2 p-3 rounded-md text-sm bg-k-danger/20 text-k-fg hover:bg-k-danger/30;
}
</style>
