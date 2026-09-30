<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader>Settings</ScreenHeader>
    </template>

    <Tabs v-if="tabs.length" class="settings-tabs" data-vue="SettingsScreen">
      <TabList>
        <TabButton
          v-for="tab in tabs"
          :id="`settingsTab-${tab.id}`"
          :key="tab.id"
          :aria-controls="`settingsPane-${tab.id}`"
          :data-testid="`settings-tab-${tab.id}`"
          :selected="currentTabId === tab.id"
          @click="currentTabId = tab.id"
        >
          <M3Icon v-if="tab.icon" :fill="currentTabId === tab.id" :name="tab.icon" :size="20" />
          {{ tab.label }}
        </TabButton>
      </TabList>

      <TabPanelContainer class="max-w-[808px]">
        <TabPanel
          v-for="tab in tabs"
          v-show="currentTabId === tab.id"
          :id="`settingsPane-${tab.id}`"
          :key="tab.id"
          :aria-labelledby="`settingsTab-${tab.id}`"
        >
          <component :is="tab.component" v-bind="tab.props" />
        </TabPanel>
      </TabPanelContainer>
    </Tabs>
  </ScreenBase>
</template>

<script lang="ts" setup>
import type { Component } from 'vue'
import { ref } from 'vue'
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'

import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import Tabs from '@/components/ui/tabs/Tabs.vue'
import TabList from '@/components/ui/tabs/TabList.vue'
import TabButton from '@/components/ui/tabs/TabButton.vue'
import TabPanelContainer from '@/components/ui/tabs/TabPanelContainer.vue'
import TabPanel from '@/components/ui/tabs/TabPanel.vue'
import YouTubeMusicSettings from '@/components/screens/settings/YouTubeMusicSettings.vue'
import LibrarySettings from '@/components/screens/settings/LibrarySettings.vue'
import ApiKeySettings from '@/components/screens/settings/ApiKeySettings.vue'

export interface SettingsTab {
  id: string
  label: string
  component: Component
  props?: Record<string, unknown>
  /** A Material Symbols name. */
  icon?: string
}

const tabs = applyFilters<SettingsTab[]>(Filter.SETTINGS_TABS, [
  { id: 'youtube-music', label: 'YouTube Music', icon: 'smart_display', component: YouTubeMusicSettings },
  { id: 'library', label: 'Library', icon: 'library_music', component: LibrarySettings },
  { id: 'api-keys', label: 'API keys', icon: 'key', component: ApiKeySettings },
])

const currentTabId = ref(tabs[0]?.id)
</script>

<style scoped>
/* The tabs run to the screen's edges. */
.settings-tabs {
  margin: 0 -24px;

  @media (max-width: 768px) {
    margin: 0 -16px;
  }
}
</style>
