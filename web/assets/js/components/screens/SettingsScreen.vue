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
import { computed, ref, watch } from 'vue'
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'
import { usePolicies } from '@/composables/usePolicies'
import { useRouter } from '@/composables/useRouter'

import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import Tabs from '@/components/ui/tabs/Tabs.vue'
import TabList from '@/components/ui/tabs/TabList.vue'
import TabButton from '@/components/ui/tabs/TabButton.vue'
import TabPanelContainer from '@/components/ui/tabs/TabPanelContainer.vue'
import TabPanel from '@/components/ui/tabs/TabPanel.vue'
import AccountSettings from '@/components/account/AccountSettings.vue'
import YouTubeMusicSettings from '@/components/screens/settings/YouTubeMusicSettings.vue'
import LibrarySettings from '@/components/screens/settings/LibrarySettings.vue'
import UsersSettings from '@/components/screens/settings/admin/UsersSettings.vue'
import SignInSettings from '@/components/screens/settings/admin/SignInSettings.vue'
import EmailSettings from '@/components/screens/settings/admin/EmailSettings.vue'

export interface SettingsTab {
  id: string
  label: string
  component: Component
  props?: Record<string, unknown>
  /** A Material Symbols name. */
  icon?: string
  /** Whether the signed-in user sees the tab; everyone does by default. */
  visible?: () => boolean
}

const { currentUserCan } = usePolicies()
const { getRouteParam, onScreenActivated } = useRouter()

const allTabs = applyFilters<SettingsTab[]>(Filter.SETTINGS_TABS, [
  { id: 'account', label: 'Account', icon: 'account_circle', component: AccountSettings },
  { id: 'youtube-music', label: 'YouTube Music', icon: 'smart_display', component: YouTubeMusicSettings },
  { id: 'library', label: 'Library', icon: 'library_music', component: LibrarySettings },
  {
    id: 'users',
    label: 'Users',
    icon: 'group',
    component: UsersSettings,
    visible: () => currentUserCan.manageUsers(),
  },
  {
    id: 'sign-in',
    label: 'Sign-in',
    icon: 'login',
    component: SignInSettings,
    visible: () => currentUserCan.manageUsers(),
  },
  {
    id: 'email',
    label: 'Email',
    icon: 'mail',
    component: EmailSettings,
    visible: () => currentUserCan.manageUsers(),
  },
])

const tabs = computed(() => allTabs.filter(tab => tab.visible?.() ?? true))

/** The tab `?tab=` names, if the user sees it. */
const requestedTab = () => {
  const wanted = getRouteParam('tab')
  return tabs.value.some(tab => tab.id === wanted) ? wanted : undefined
}

const currentTabId = ref(requestedTab() ?? tabs.value[0]?.id)

onScreenActivated('Settings', () => {
  const wanted = requestedTab()
  if (wanted) {
    currentTabId.value = wanted
  }
})

watch(tabs, visible => {
  if (!visible.some(tab => tab.id === currentTabId.value)) {
    currentTabId.value = visible[0]?.id
  }
})
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
