<template>
  <input v-model="value" :checked="value" class="k-switch" type="checkbox" v-bind="$attrs" />
</template>

<script lang="ts" setup>
const value = defineModel<boolean>()
</script>
