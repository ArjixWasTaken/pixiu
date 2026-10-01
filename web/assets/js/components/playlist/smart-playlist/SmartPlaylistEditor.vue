<template>
  <div>
    <M3Tabs v-model="tab" :tabs class="tabs" secondary />
    <div class="panels">
      <PlaylistDetails
        v-show="tab === 'details'"
        v-model:description="description"
        v-model:folder-id="folderId"
        v-model:folder-name="folderName"
        v-model:name="name"
        data-tab="details"
      />
      <SmartPlaylistRules v-show="tab === 'rules'" v-model="ruleGroups" data-tab="rules" />
    </div>
  </div>
</template>

<script lang="ts" setup>
import type { M3Tab } from '@/components/m3/M3Tabs.vue'

import M3Tabs from '@/components/m3/M3Tabs.vue'
import PlaylistDetails from '@/components/playlist/PlaylistDetails.vue'
import SmartPlaylistRules from '@/components/playlist/smart-playlist/SmartPlaylistRules.vue'

defineProps<{ tabs: M3Tab[] }>()

const tab = defineModel<string>('tab', { required: true })
const name = defineModel<string>('name', { required: true })
const description = defineModel<string>('description', { required: true })
const folderId = defineModel<PlaylistFolder['id'] | null | undefined>('folderId', { required: true })
const folderName = defineModel<string | null>('folderName', { default: null })
const ruleGroups = defineModel<SmartPlaylistRuleGroup[]>('ruleGroups', { required: true })
</script>

<style scoped>
.tabs {
  margin: 0 -24px 16px;
  border-bottom: 1px solid var(--schemes-outline-variant);
}

/* One size for both tabs: switching does not resize the dialog. */
.panels {
  height: min(440px, calc(100dvh - 280px));
  overflow-y: auto;
  padding-top: 8px;
}
</style>
