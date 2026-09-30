<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center gap-3 text-(--schemes-on-surface-variant)">
      <M3Divider class="flex-1" />
      <span class="m3-label-medium">or</span>
      <M3Divider class="flex-1" />
    </div>
    <M3Button :href class="w-full" data-testid="sso-button" icon="key" variant="outlined">
      Sign in with {{ name }}
    </M3Button>
  </div>
</template>

<script lang="ts" setup>
import M3Button from '@/components/m3/M3Button.vue'
import M3Divider from '@/components/m3/M3Divider.vue'

defineProps<{
  /** The provider, as the admin named it. */
  name: string
  /** Where signing in with it starts. */
  href: string
}>()
</script>
