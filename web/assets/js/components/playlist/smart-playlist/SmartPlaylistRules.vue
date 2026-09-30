<template>
  <div class="flex flex-col gap-4">
    <p class="m3-body-medium text-(--schemes-on-surface-variant)">
      A song is in the playlist when it matches every rule of a group. Without rules, the playlist is empty.
    </p>

    <SmartPlaylistRuleGroup
      v-for="(group, index) in groups"
      :key="group.id"
      :group
      :is-first-group="index === 0"
      @remove="removeGroup(index)"
      @update:group="setGroup(index, $event)"
    />

    <M3Button class="self-start" icon="add" variant="tonal" @click.prevent="addGroup">
      {{ groups.length ? 'Add another group' : 'Add a group' }}
    </M3Button>
  </div>
</template>

<script lang="ts" setup>
import { playlistStore } from '@/stores/playlistStore'

import M3Button from '@/components/m3/M3Button.vue'
import SmartPlaylistRuleGroup from '@/components/playlist/smart-playlist/SmartPlaylistRuleGroup.vue'

const groups = defineModel<SmartPlaylistRuleGroup[]>({ required: true })

const setGroup = (index: number, group: SmartPlaylistRuleGroup) =>
  (groups.value = groups.value.map((current, at) => (at === index ? group : current)))

const removeGroup = (index: number) => (groups.value = groups.value.filter((_, at) => at !== index))

const addGroup = () => (groups.value = [...groups.value, playlistStore.createEmptySmartPlaylistRuleGroup()])
</script>
