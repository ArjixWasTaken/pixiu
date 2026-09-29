<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader>Settings</ScreenHeader>
    </template>

    <Tabs v-if="tabs.length" class="-mx-6">
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
          {{ tab.label }}
        </TabButton>
      </TabList>

      <TabPanelContainer class="scroll-mask-y">
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
}

const tabs = applyFilters<SettingsTab[]>(Filter.SETTINGS_TABS, [
  { id: 'youtube-music', label: 'YouTube Music', component: YouTubeMusicSettings },
  { id: 'library', label: 'Library', component: LibrarySettings },
  { id: 'api-keys', label: 'API Keys', component: ApiKeySettings },
])

const currentTabId = ref(tabs[0]?.id)
</script>
