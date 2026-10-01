<template>
  <DropdownMenuSeparator as="li" class="separator" />
</template>

<script setup lang="ts">
import { DropdownMenuSeparator } from 'reka-ui'
</script>

<style scoped>
.separator + .separator {
  display: none;
}
</style>
