<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader>Preferences</ScreenHeader>
    </template>

    <Tabs class="-mx-6 shrink-0" data-vue="ProfileScreen">
      <TabList sticky>
        <TabButton
          :selected="currentTab === 'preferences'"
          aria-controls="profilePanePreferences"
          @click="currentTab = 'preferences'"
        >
          Preferences
        </TabButton>
        <TabButton :selected="currentTab === 'themes'" aria-controls="profilePaneThemes" @click="currentTab = 'themes'">
          Themes
        </TabButton>
      </TabList>

      <TabPanelContainer :scroll="false">
        <TabPanel
          v-if="currentTab === 'preferences'"
          id="profilePanePreferences"
          aria-labelledby="profilePanePreferences"
        >
          <PreferencesForm />
        </TabPanel>

        <TabPanel v-if="currentTab === 'themes'" id="profilePaneThemes" aria-labelledby="profilePaneThemes">
          <ThemeList />
        </TabPanel>
      </TabPanelContainer>
    </Tabs>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { ref, watch } from 'vue'
import { useLocalStorage } from '@/composables/useLocalStorage'
import { defineAsyncComponent } from '@/utils/helpers'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import TabButton from '@/components/ui/tabs/TabButton.vue'
import TabList from '@/components/ui/tabs/TabList.vue'
import TabPanelContainer from '@/components/ui/tabs/TabPanelContainer.vue'
import TabPanel from '@/components/ui/tabs/TabPanel.vue'
import Tabs from '@/components/ui/tabs/Tabs.vue'

const PreferencesForm = defineAsyncComponent(() => import('@/components/profile-preferences/PreferencesForm.vue'))
const ThemeList = defineAsyncComponent(() => import('@/components/profile-preferences/theme/ThemePreferences.vue'))

const { get, set } = useLocalStorage()

const currentTab = ref(get<'preferences' | 'themes'>('profileScreenTab', 'preferences'))

// An older tab name may be stored; fall back to preferences.
if (!['preferences', 'themes'].includes(currentTab.value!)) {
  currentTab.value = 'preferences'
}

watch(currentTab, tab => {
  set('profileScreenTab', tab)
  // Another tab starts at its top, not where the last one was scrolled to.
  document.querySelector('[data-vue="ProfileScreen"]')?.closest('.screen-body')?.scrollTo?.({ top: 0 })
})
</script>
