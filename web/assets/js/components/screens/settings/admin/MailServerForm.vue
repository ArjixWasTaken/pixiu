<template>
  <form class="flex flex-col gap-4" data-testid="mail-server-form" @submit.prevent="handleSubmit">
    <div class="grid md:grid-cols-[1fr_120px] gap-4">
      <M3TextField
        v-model="data.host"
        autocomplete="off"
        label="Server"
        name="host"
        placeholder="smtp.example.com"
        required
      />
      <M3TextField v-model="data.port" label="Port" min="1" max="65535" name="port" required type="number" />
    </div>

    <div class="flex flex-col gap-2">
      <span class="m3-label-large text-(--schemes-on-surface-variant)">Security</span>
      <M3SegmentedButton :model-value="data.security" :segments="securities" @update:model-value="setSecurity" />
      <span class="m3-body-small text-(--schemes-on-surface-variant)">{{ securityNote }}</span>
    </div>

    <div class="grid md:grid-cols-2 gap-4">
      <M3TextField v-model="data.username" autocomplete="off" label="Username (optional)" name="username" />
      <M3TextField
        v-model="data.password"
        :supporting-text="passwordNote"
        autocomplete="new-password"
        label="Password"
        name="password"
        type="password"
      />
    </div>
    <M3Button
      v-if="server?.password_set && !forgetPassword && !data.password"
      class="self-start"
      variant="text"
      @click.prevent="forgetPassword = true"
    >
      Forget the saved password
    </M3Button>

    <M3TextField
      v-model="data.from"
      label="Sender"
      name="from"
      placeholder="píxiū <no-reply@example.com>"
      required
      supporting-text="The address emails come from, with a name if you like."
    />

    <div class="flex flex-wrap justify-end gap-3">
      <M3Button v-if="server" variant="outlined" @click.prevent="$emit('remove')">Remove</M3Button>
      <M3Button type="submit">Save</M3Button>
    </div>
  </form>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from 'vue'
import type { MailSecurity, MailServer, MailServerForm } from '@/services/serverSettingsService'
import { useForm } from '@/composables/useForm'

import M3Button from '@/components/m3/M3Button.vue'
import M3SegmentedButton from '@/components/m3/M3SegmentedButton.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{ server: MailServer | null }>()
const emit = defineEmits<{ (e: 'save', form: MailServerForm): void; (e: 'remove'): void }>()

const securities = [
  { id: 'starttls', label: 'STARTTLS' },
  { id: 'tls', label: 'TLS' },
  { id: 'none', label: 'None' },
]

/** The usual port of each. */
const PORTS: Record<MailSecurity, number> = { starttls: 587, tls: 465, none: 25 }

const forgetPassword = ref(false)

const initial = () => ({
  host: props.server?.host ?? '',
  port: props.server?.port ?? PORTS.starttls,
  security: props.server?.security ?? ('starttls' as MailSecurity),
  username: props.server?.username ?? '',
  password: '',
  from: props.server?.from ?? '',
})

const { data, handleSubmit } = useForm<ReturnType<typeof initial>>({
  initialValues: initial(),
  useOverlay: false,
  validator: ({ host, from }) => host.trim() !== '' && from.trim() !== '',
  onSubmit: async ({ host, port, security, username, password, from }) =>
    emit('save', {
      host,
      port: Number(port),
      security,
      username,
      // Nothing typed keeps the saved password, unless it is to go.
      password: password || (forgetPassword.value ? '' : undefined),
      from,
    }),
})

/** Picking a way to secure mail picks its usual port, unless another was chosen. */
const setSecurity = (security?: string) => {
  if (!security) {
    return
  }

  if (Object.values(PORTS).includes(Number(data.port))) {
    data.port = PORTS[security as MailSecurity]
  }
  data.security = security as MailSecurity
}

const securityNote = computed(
  () =>
    ({
      starttls: 'Encrypted once connected; usually port 587.',
      tls: 'Encrypted from the start; usually port 465.',
      none: 'Not encrypted: only for a mail server on the same machine or network.',
    })[data.security],
)

const passwordNote = computed(() => {
  if (forgetPassword.value) {
    return 'The saved password goes when you save.'
  }

  return props.server?.password_set ? 'Saved. Type a new one to replace it.' : undefined
})

watch(
  () => props.server,
  () => {
    Object.assign(data, initial())
    forgetPassword.value = false
  },
)
</script>
