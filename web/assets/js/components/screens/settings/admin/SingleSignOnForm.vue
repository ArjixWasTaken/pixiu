<template>
  <form class="flex flex-col gap-4" data-testid="single-sign-on-form" @submit.prevent="handleSubmit">
    <div class="redirect flex flex-col gap-1" data-testid="redirect-uri">
      <span class="m3-label-large">Redirect URI</span>
      <template v-if="redirectUri">
        <span class="flex items-center gap-2">
          <code class="flex-1 m3-body-medium break-all">{{ redirectUri }}</code>
          <M3IconButton v-if="canCopy" icon="content_copy" label="Copy the redirect URI" @click="copy" />
        </span>
        <span class="m3-body-small">Give the provider this when you register píxiū as a client there.</span>
      </template>
      <span v-else class="m3-body-medium">
        Save the public address above first: the provider sends people back to it.
      </span>
    </div>

    <div class="grid md:grid-cols-2 gap-4">
      <M3TextField
        v-model="data.name"
        label="Name"
        name="name"
        required
        supporting-text="For the sign-in button: “Sign in with Authelia”, say."
      />
      <M3TextField
        v-model="data.issuer"
        label="Issuer"
        name="issuer"
        required
        :supporting-text="`The provider’s address, like https://\u2060sso.example.com`"
        type="url"
      />
      <M3TextField v-model="data.client_id" autocomplete="off" label="Client ID" name="client_id" required />
      <M3TextField
        v-model="data.client_secret"
        :required="!oidc"
        :supporting-text="oidc ? 'Saved. Type a new one to replace it.' : undefined"
        autocomplete="new-password"
        label="Client secret"
        name="client_secret"
        type="password"
      />
    </div>
    <M3TextField
      v-model="data.scopes"
      label="Scopes (optional)"
      name="scopes"
      supporting-text="Asked for besides openid; profile and email when left empty."
    />

    <div class="flex flex-wrap justify-end gap-3">
      <M3Button v-if="oidc" variant="text" @click.prevent="$emit('remove')">Remove</M3Button>
      <M3Button :disabled="testing" icon="network_check" variant="outlined" @click.prevent="test">Test</M3Button>
      <M3Button type="submit">Save</M3Button>
    </div>
  </form>
</template>

<script lang="ts" setup>
import { watch } from 'vue'
import type { SingleSignOn, SingleSignOnForm } from '@/services/serverSettingsService'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{
  oidc: SingleSignOn | null
  redirectUri: string | null
  /** A test is running. */
  testing: boolean
}>()

const emit = defineEmits<{
  (e: 'save', form: SingleSignOnForm): void
  (e: 'test', form: SingleSignOnForm): void
  (e: 'remove'): void
  (e: 'copied'): void
}>()

const canCopy = typeof navigator !== 'undefined' && Boolean(navigator.clipboard?.writeText)

const initial = () => ({
  name: props.oidc?.name ?? '',
  issuer: props.oidc?.issuer ?? '',
  client_id: props.oidc?.client_id ?? '',
  client_secret: '',
  scopes: props.oidc?.scopes.join(' ') ?? '',
})

/** The form as the API takes it: an empty secret keeps the saved one. */
const asForm = (values: ReturnType<typeof initial>): SingleSignOnForm => ({
  name: values.name.trim(),
  issuer: values.issuer.trim(),
  client_id: values.client_id.trim(),
  client_secret: values.client_secret || undefined,
  scopes: values.scopes.split(/\s+/).filter(Boolean),
})

const { data, handleSubmit } = useForm<ReturnType<typeof initial>>({
  initialValues: initial(),
  useOverlay: false,
  validator: values => values.name.trim() !== '' && values.issuer.trim() !== '' && values.client_id.trim() !== '',
  onSubmit: async values => emit('save', asForm(values)),
})

const test = () => emit('test', asForm(data))

const copy = async () => {
  if (props.redirectUri) {
    await navigator.clipboard.writeText(props.redirectUri)
    emit('copied')
  }
}

watch(
  () => props.oidc,
  () => Object.assign(data, initial()),
)
</script>

<style scoped>
.redirect {
  padding: 12px 16px;
  border-radius: 12px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);

  code {
    color: var(--schemes-on-surface);
  }
}
</style>
