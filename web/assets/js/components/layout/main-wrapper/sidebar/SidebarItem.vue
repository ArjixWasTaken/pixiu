<template>
  <li :class="{ active }" class="sidebar-item" data-testid="sidebar-item">
    <a
      :aria-current="active ? 'page' : undefined"
      :href="props.href"
      class="m3-state m3-label-large"
      @click.prevent="onClick"
      @dblclick.prevent="onDblClick"
    >
      <slot name="icon">
        <M3Icon v-if="icon" :class="{ spin }" :fill="active" :name="icon" />
      </slot>

      <span class="label">
        <slot />
      </span>

      <span v-if="$slots.badge" class="badge" data-testid="sidebar-item-badge">
        <slot name="badge" />
      </span>

      <slot name="trailing" />
    </a>
  </li>
</template>

<script lang="ts" setup>
import { useRouter } from '@/composables/useRouter'
import { eventBus } from '@/utils/eventBus'
import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(
  defineProps<{
    href?: string | undefined
    active?: boolean
    icon?: string
    spin?: boolean
  }>(),
  {
    active: false,
    spin: false,
  },
)

const emit = defineEmits<{ dblclick: [] }>()

const { go } = useRouter()

let clickTimer = 0

const navigate = () => {
  if (props.href) {
    go(props.href)
    eventBus.emit('TOGGLE_SIDEBAR')
  }
}

const onClick = () => {
  clearTimeout(clickTimer)
  clickTimer = window.setTimeout(navigate, 150)
}

const onDblClick = () => {
  clearTimeout(clickTimer)
  emit('dblclick')
}
</script>

<style lang="postcss" scoped>
.sidebar-item {
  list-style: none;

  a {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 56px;
    padding: 0 24px 0 16px;
    border-radius: 9999px;
    color: var(--schemes-on-surface-variant);
    text-decoration: none;

    &:hover {
      color: var(--schemes-on-surface);
    }
  }

  &.active a {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}

.label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badge {
  flex-shrink: 0;
}

.spin {
  animation: m3-spin 2s linear infinite;
}
</style>
