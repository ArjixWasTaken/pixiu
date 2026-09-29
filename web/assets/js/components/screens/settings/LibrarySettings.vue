<template>
  <div v-if="settings" class="flex flex-col gap-10 md:w-2/3">
    <LayoutSettingGroup :layout="settings.layout" @changed="fetchSettings" />
    <MusicBrainzSettingGroup :unlooked="settings.albums_not_looked_up" @changed="fetchSettings" />
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { Settings } from '@/services/huntingService'
import { useErrorHandler } from '@/composables/useErrorHandler'

import LayoutSettingGroup from '@/components/screens/settings/LayoutSettingGroup.vue'
import MusicBrainzSettingGroup from '@/components/screens/settings/MusicBrainzSettingGroup.vue'

const { handleHttpError } = useErrorHandler('dialog')

const settings = ref<Settings | null>(null)

const fetchSettings = async () => {
  try {
    settings.value = await huntingService.settings()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(fetchSettings)
</script>
