<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Not found</ScreenHeader>
    </template>

    <ScreenEmptyState>
      <template #icon>
        <M3Icon :size="64" name="explore_off" />
      </template>

      There’s nothing here.
      <span class="secondary block">The link may be old, or what it led to was deleted.</span>
      <M3Button :href="url('home')" class="mt-4" icon="home" variant="tonal">Go to Home</M3Button>
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { useRouter } from '@/composables/useRouter'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'

const { url } = useRouter()
</script>
