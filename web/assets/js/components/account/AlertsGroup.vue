<template>
  <SettingGroup data-testid="alerts-group">
    <template #title>Email alerts</template>
    <template #subtitle>
      {{
        preferences?.deliverable === false
          ? 'Alerts go to a confirmed email address, once email is set up on this server.'
          : 'píxiū emails you when something needs you, once per problem.'
      }}
    </template>

    <ul v-if="preferences" class="flex flex-col">
      <li v-for="alert in ALERTS" :key="alert.kind">
        <label class="flex items-center gap-3 py-1 cursor-pointer">
          <M3Checkbox
            :model-value="preferences.alerts[alert.kind]"
            :name="alert.kind"
            @update:model-value="set(alert.kind, $event)"
          />
          <span class="flex flex-col">
            <span class="m3-body-large text-(--schemes-on-surface)">{{ alert.label }}</span>
            <span class="m3-body-medium text-(--schemes-on-surface-variant)">{{ alert.detail }}</span>
          </span>
        </label>
      </li>
    </ul>
  </SettingGroup>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { accountService } from '@/services/accountService'
import type { AlertKind, AlertPreferences } from '@/services/accountService'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Checkbox from '@/components/m3/M3Checkbox.vue'
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'

const ALERTS: { kind: AlertKind; label: string; detail: string }[] = [
  {
    kind: 'youtube_music_expired',
    label: 'YouTube Music signs me out',
    detail: 'Downloads wait until you sign in again.',
  },
  {
    kind: 'watch_failing',
    label: 'A watch keeps failing',
    detail: 'After three failed syncs in a row.',
  },
]

const { handleHttpError } = useErrorHandler('dialog')

const preferences = ref<AlertPreferences | null>(null)

const set = async (kind: AlertKind, on: boolean) => {
  try {
    preferences.value = await accountService.setAlerts({ [kind]: on })
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

onMounted(async () => {
  try {
    preferences.value = await accountService.alerts()
  } catch (error: unknown) {
    handleHttpError(error)
  }
})
</script>
