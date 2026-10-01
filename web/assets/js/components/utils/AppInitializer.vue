<template>
  <slot />
</template>

<script lang="ts" setup>
import { onMounted } from 'vue'
import { useAuthorization } from '@/composables/useAuthorization'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { useOverlay } from '@/composables/useOverlay'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { shouldWarnUponWindowUnload as shouldWarnAboutOfflineCaching } from '@/composables/useOfflinePlayback'
import { useUpload } from '@/composables/useUpload'

const commonStore = useCommonStore()
const preferences = usePreferenceStore()

const emits = defineEmits<{
  (e: 'success'): void
  (e: 'error', err: unknown): void
}>()

const { showOverlay, hideOverlay } = useOverlay()
const { currentUser } = useAuthorization()
const { handleHttpError } = useErrorHandler()
const { unfinishedUploadCount } = useUpload()

onMounted(async () => {
  showOverlay({ message: 'Just a little patience…' })

  try {
    await commonStore.init()

    window.addEventListener('beforeunload', (e: BeforeUnloadEvent) => {
      if (unfinishedUploadCount.value > 0 || shouldWarnAboutOfflineCaching() || preferences.confirm_before_closing) {
        e.preventDefault()
        e.returnValue = ''
      }
    })

    emits('success')
  } catch (error: unknown) {
    handleHttpError(error)
    emits('error', error)
  } finally {
    hideOverlay()
  }
})
</script>
