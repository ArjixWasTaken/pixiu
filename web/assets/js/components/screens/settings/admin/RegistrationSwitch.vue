<template>
  <div class="flex flex-col gap-2" data-testid="registration-switch">
    <label :class="{ unavailable: !mailReady }" class="flex items-center gap-3">
      <M3Checkbox
        :disabled="!mailReady"
        :model-value="open && mailReady"
        name="registration_open"
        @update:model-value="$emit('toggle', $event)"
      />
      <span class="flex flex-col">
        <span class="m3-body-large text-(--schemes-on-surface)">Anyone may ask for an account</span>
        <span class="m3-body-medium text-(--schemes-on-surface-variant)">
          The sign-in screen offers it. Admins approve or deny each request; approved, people confirm their email and
          sign in.
        </span>
      </span>
    </label>
    <p v-if="!mailReady" class="m3-body-medium text-(--schemes-on-surface-variant)">
      Needs email, which tells people how their request went: set up a mail server under Email, and the public address
      above.
    </p>
  </div>
</template>

<script lang="ts" setup>
import M3Checkbox from '@/components/m3/M3Checkbox.vue'

defineProps<{ open: boolean; mailReady: boolean }>()
defineEmits<{ (e: 'toggle', open: boolean): void }>()
</script>

<style scoped>
.unavailable {
  opacity: 0.6;
}
</style>
