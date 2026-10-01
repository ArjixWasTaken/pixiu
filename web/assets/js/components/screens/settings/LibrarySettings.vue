<template>
  <div v-if="settings" class="flex flex-col gap-6">
    <MusicBrainzSettingGroup :unlooked="settings.albums_not_looked_up" @changed="refetch" />
  </div>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import { useErrorHandler } from '@/composables/useErrorHandler'

import MusicBrainzSettingGroup from '@/components/screens/settings/MusicBrainzSettingGroup.vue'

const { handleHttpError } = useErrorHandler('dialog')

const {
  data: settings,
  error,
  refetch,
} = useQuery({ queryKey: ['settings'], queryFn: () => huntingService.settings() })
watch(error, error => error && handleHttpError(error))
</script>
