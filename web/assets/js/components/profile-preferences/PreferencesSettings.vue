<template>
  <div class="flex flex-col gap-6">
    <SettingGroup>
      <template #title>Playback</template>
      <PreferencesForm />
    </SettingGroup>
    <SettingGroup>
      <template #title>Theme</template>
      <ThemePreferences />
    </SettingGroup>
  </div>
</template>

<script lang="ts" setup>
import SettingGroup from '@/components/screens/settings/SettingGroup.vue'
import PreferencesForm from '@/components/profile-preferences/PreferencesForm.vue'
import ThemePreferences from '@/components/profile-preferences/theme/ThemePreferences.vue'
</script>
