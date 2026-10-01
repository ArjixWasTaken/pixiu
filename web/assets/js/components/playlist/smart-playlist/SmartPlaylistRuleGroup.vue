<template>
  <M3Card class="flex flex-col gap-4 p-4" data-testid="smart-playlist-rule-group" variant="outlined">
    <h3 class="m3-title-small">
      <template v-if="isFirstGroup">Songs that match <strong>all</strong> of these</template>
      <template v-else>Or songs that match <strong>all</strong> of these</template>
    </h3>

    <SmartPlaylistRule
      v-for="(rule, index) in group.rules"
      :key="rule.id"
      :rule
      @remove="removeRule(index)"
      @update:rule="setRule(index, $event)"
    />

    <M3Button class="self-start" icon="add" size="s" variant="text" @click.prevent="addRule">Add a rule</M3Button>
  </M3Card>
</template>

<script lang="ts" setup>
import { playlistStore } from '@/stores/playlistStore'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import SmartPlaylistRule from '@/components/playlist/smart-playlist/SmartPlaylistRule.vue'

const props = defineProps<{ group: SmartPlaylistRuleGroup; isFirstGroup: boolean }>()

const emit = defineEmits<{
  (e: 'update:group', group: SmartPlaylistRuleGroup): void
  /** Its last rule went: the group goes too. */
  (e: 'remove'): void
}>()

const setRules = (rules: SmartPlaylistRule[]) =>
  rules.length ? emit('update:group', { ...props.group, rules }) : emit('remove')

const setRule = (index: number, rule: SmartPlaylistRule) =>
  setRules(props.group.rules.map((current, at) => (at === index ? rule : current)))

const removeRule = (index: number) => setRules(props.group.rules.filter((_, at) => at !== index))

const addRule = () => setRules([...props.group.rules, playlistStore.createEmptySmartPlaylistRule()])
</script>
