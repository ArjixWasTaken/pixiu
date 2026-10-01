<template>
  <a v-if="expired" :href="url('settings')" class="notice">
    <M3Icon name="warning" />
    <span>YouTube Music signed píxiū out. Log in again.</span>
  </a>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { huntingStore } from '@/stores/huntingStore'
import { useRouter } from '@/composables/useRouter'
import M3Icon from '@/components/m3/M3Icon.vue'

const { url } = useRouter()

const expired = computed(() => huntingStore.state.session === 'expired')
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.notice {
  @apply flex items-start gap-2 mx-6 mb-2 p-3 rounded-md text-sm bg-(--schemes-error)/20 text-(--schemes-on-surface) hover:bg-(--schemes-error)/30;
}
</style>
