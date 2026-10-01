<template>
  <ScreenEmptyState role="alert">
    <template #icon>
      <M3Icon :size="64" name="cloud_off" />
    </template>

    Couldn’t load the {{ what }}.
    <span class="secondary block">Check the connection, then try again.</span>
    <M3Button class="mt-4 self-center" icon="refresh" variant="tonal" @click="emit('retry')">Try again</M3Button>
  </ScreenEmptyState>
</template>

<script lang="ts" setup>
import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'

/** What a screen shows when its list couldn't load and there's nothing to show instead. */
defineProps<{
  /** What the list holds, e.g. "albums". */
  what: string
}>()

const emit = defineEmits<{ retry: [] }>()
</script>
