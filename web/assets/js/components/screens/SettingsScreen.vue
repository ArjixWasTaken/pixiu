<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Settings</ScreenHeader>
    </template>

    <div v-if="tabs.length" class="settings-tabs" data-vue="SettingsScreen">
      <M3Tabs v-model="currentTabId" :tabs class="tab-bar" id-prefix="settings" scrollable secondary sticky />

      <div class="panels">
        <div
          v-for="tab in tabs"
          v-show="currentTabId === tab.id"
          :id="`settings-panel-${tab.id}`"
          :key="tab.id"
          :aria-labelledby="`settings-tab-${tab.id}`"
          :class="{ columns: tab.columns }"
          class="panel"
          role="tabpanel"
        >
          <component :is="tab.component" v-bind="tab.props" />
        </div>
      </div>
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import type { Component } from 'vue'
import { computed, ref, watch } from 'vue'
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'
import { usePolicies } from '@/composables/usePolicies'
import { useRouter } from '@/composables/useRouter'

import M3Tabs from '@/components/m3/M3Tabs.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import AccountSettings from '@/components/account/AccountSettings.vue'
import PreferencesSettings from '@/components/profile-preferences/PreferencesSettings.vue'
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
  /** Tabs of a group follow its name ("Server"); a tab without one is the user's own. */
  group?: string
  /** On a wide screen, its groups sit in two columns. */
  columns?: boolean
}

const { currentUserCan } = usePolicies()
const { getRouteParam, onScreenActivated } = useRouter()

const allTabs = applyFilters<SettingsTab[]>(Filter.SETTINGS_TABS, [
  { id: 'account', label: 'Account', icon: 'account_circle', component: AccountSettings, columns: true },
  { id: 'preferences', label: 'Preferences', icon: 'tune', component: PreferencesSettings, columns: true },
  { id: 'youtube-music', label: 'YouTube Music', icon: 'smart_display', component: YouTubeMusicSettings },
  { id: 'library', label: 'Library', icon: 'library_music', component: LibrarySettings },
  {
    id: 'users',
    label: 'Users',
    icon: 'group',
    component: UsersSettings,
    group: 'Server',
    visible: () => currentUserCan.manageUsers(),
  },
  {
    id: 'sign-in',
    label: 'Sign-in',
    icon: 'login',
    component: SignInSettings,
    group: 'Server',
    columns: true,
    visible: () => currentUserCan.manageUsers(),
  },
  {
    id: 'email',
    label: 'Email',
    icon: 'mail',
    component: EmailSettings,
    group: 'Server',
    columns: true,
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

// Another tab starts at its top, not where the last one was scrolled to.
watch(currentTabId, () =>
  document.querySelector('[data-vue="SettingsScreen"]')?.closest('.screen-body')?.scrollTo?.({ top: 0 }),
)

watch(tabs, visible => {
  if (!visible.some(tab => tab.id === currentTabId.value)) {
    currentTabId.value = visible[0]?.id
  }
})
</script>

<style scoped>
/* The tab bar runs to the screen's edges. */
.settings-tabs {
  /* As tall as the settings, so the tab bar stays stuck all the way down. */
  flex-shrink: 0;
}

.tab-bar {
  margin: 0 -24px;
  padding: 0 24px;
  border-bottom: 1px solid var(--schemes-outline-variant);

  @media (max-width: 768px) {
    margin: 0 -16px;
    padding: 0 16px;
  }
}

.panels {
  padding: var(--m3-gutter) 0;
}

.panel {
  max-width: 808px;
}

/* Wide enough for two: the groups flow down one column, then the next. */
@media (min-width: 1280px) {
  .panel.columns {
    max-width: 1280px;

    > :deep(.flex-col) {
      display: block;
      columns: 2;
      column-gap: var(--m3-gutter);

      > * {
        break-inside: avoid;
        margin-bottom: var(--m3-gutter);
      }
    }
  }
}
</style>
