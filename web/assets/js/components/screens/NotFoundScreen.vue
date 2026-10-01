<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">Not found</ScreenHeader>
    </template>

    <ScreenEmptyState>
      <template #icon>
        <M3Icon :size="64" name="explore_off" />
      </template>

      The requested content cannot be found.
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
</script>
