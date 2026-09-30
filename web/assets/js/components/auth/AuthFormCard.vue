<template>
  <M3Card :class="{ error: failed }" class="auth-card" variant="elevated">
    <form class="flex flex-col gap-5 px-6 pt-8 pb-6" @submit.prevent="$emit('submit')">
      <div class="flex flex-col items-center gap-3 text-center">
        <img :src="logo" alt="" height="72" width="72" />
        <slot name="title" />
      </div>

      <slot />
    </form>
  </M3Card>
</template>

<script lang="ts" setup>
import { useBranding } from '@/composables/useBranding'

import M3Card from '@/components/m3/M3Card.vue'

withDefaults(defineProps<{ failed?: boolean }>(), { failed: false })
defineEmits<{ (e: 'submit'): void }>()

const { logo } = useBranding()
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

@keyframes shake {
  8%,
  41% {
    transform: translateX(-10px);
  }
  25%,
  58% {
    transform: translateX(10px);
  }
  75% {
    transform: translateX(-5px);
  }
  92% {
    transform: translateX(5px);
  }
  0%,
  100% {
    transform: translateX(0);
  }
}

.auth-card {
  width: 380px;
  max-width: 100%;
  border-radius: 28px;
}

.error {
  animation: shake 0.5s;
}
</style>
