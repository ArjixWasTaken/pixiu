<template>
  <span v-if="allowsUpload" class="secondary block">
    <a :href="url('upload')" class="inline">Upload some music</a> or find some on
    <a :href="url('hunt')" class="inline">Discover</a> to get started.
  </span>
</template>

<script lang="ts" setup>
import { useRouter } from '@/composables/useRouter'
import { useUpload } from '@/composables/useUpload'

const { url } = useRouter()
const { allowsUpload } = useUpload()
</script>
