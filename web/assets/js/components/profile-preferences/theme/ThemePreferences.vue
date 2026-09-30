<template>
  <section class="flex flex-col gap-6">
    <div class="flex items-center gap-4 flex-wrap">
      <span class="m3-title-medium text-(--schemes-on-surface)">Mode</span>
      <M3SegmentedButton v-model="mode" :segments="modes" data-testid="dark-mode" />
    </div>
    <ThemeList :themes data-testid="built-in-themes" />
  </section>
</template>

<script lang="ts" setup>
import { computed, toRef } from 'vue'
import { themeStore } from '@/stores/themeStore'
import { preferenceStore } from '@/stores/preferenceStore'

import M3SegmentedButton from '@/components/m3/M3SegmentedButton.vue'
import ThemeList from '@/components/profile-preferences/theme/ThemeList.vue'

const themes = toRef(themeStore.state, 'themes')

const modes = [
  { id: 'dark', label: 'Dark', icon: 'dark_mode' },
  { id: 'light', label: 'Light', icon: 'light_mode' },
]

const mode = computed({
  get: () => ((preferenceStore.state.dark_mode ?? true) ? 'dark' : 'light'),
  set: value => themeStore.setDarkMode(value === 'dark'),
})
</script>
