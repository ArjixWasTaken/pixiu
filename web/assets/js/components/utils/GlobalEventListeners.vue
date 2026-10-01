<template>
  <slot />
</template>

<script lang="ts" setup>
import { onMounted } from 'vue'
import { useRouter } from '@/composables/useRouter'
import { eventBus } from '@/utils/eventBus'
import { authService } from '@/services/authService'
import { useHuntingStore } from '@/stores/huntingStore'
import { forceReloadWindow } from '@/utils/helpers'

const huntingStore = useHuntingStore()

let go: ReturnType<typeof useRouter>['go']

onMounted(() => {
  go = useRouter().go
})

eventBus.on('LOG_OUT', async () => {
  huntingStore.disconnect()
  await authService.logout()
  go('/')
  forceReloadWindow()
})
</script>
