<template>
  <M3Card
    :aria-pressed="isCurrentTheme"
    :class="{ current: isCurrentTheme }"
    :title="isCurrentTheme ? `${theme.name} (current scheme)` : `Use the ${theme.name} scheme`"
    class="theme flex items-center gap-3 w-full h-full p-3 text-left"
    data-testid="theme-card"
    interactive
    tag="button"
    type="button"
    variant="outlined"
    @click="onClick"
  >
    <span :style="{ background: theme.thumbnail_color }" class="swatch" />
    <span class="m3-title-medium flex-1">{{ theme.name }}</span>
    <M3Icon v-if="isCurrentTheme" class="text-(--schemes-primary)" fill name="check_circle" />
  </M3Card>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { useThemeStore } from '@/stores/themeStore'
import M3Card from '@/components/m3/M3Card.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const themeStore = useThemeStore()

const props = defineProps<{ theme: Theme }>()

const { theme } = toRefs(props)

const isCurrentTheme = computed(() => themeStore.isCurrentTheme(theme.value))

const onClick = () => themeStore.setTheme(theme.value)
</script>

<style scoped>
.swatch {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  flex-shrink: 0;
  border: 1px solid var(--schemes-outline-variant);
}

.current {
  border-color: var(--schemes-primary);
}
</style>
