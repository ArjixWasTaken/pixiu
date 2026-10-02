<template>
  <section v-if="details" class="flex flex-col gap-4">
    <h3 class="text-xl">MusicBrainz</h3>

    <p v-if="details.enrichment === 'matched'">
      Matched to
      <a :href="`https://musicbrainz.org/release/${details.mbid}`" class="release-link" rel="noopener" target="_blank"
        >“{{ details.title }}” by {{ details.artist }}</a
      >
      on MusicBrainz{{ details.enriched_at ? ` ${timeAgo(details.enriched_at)}` : '' }}; its tags follow it.
    </p>
    <p v-else-if="details.enrichment === 'review'" class="text-(--schemes-tertiary)">
      píxiū isn’t sure which release this is. Pick one below, or paste a MusicBrainz release link.
    </p>
    <p v-else-if="details.enrichment === 'unmatched'">MusicBrainz knows nothing like it.</p>
    <p v-else class="text-(--schemes-on-surface-variant)">Not looked up yet.</p>

    <ul v-if="details.candidates.length" class="flex flex-col gap-2">
      <li
        v-for="candidate in details.candidates"
        :key="candidate.id"
        class="flex items-center gap-4 p-3 rounded-lg bg-(--schemes-surface-container-high)"
      >
        <span class="w-12 text-center tabular-nums text-(--schemes-on-surface-variant)"
          >{{ Math.round(candidate.score * 100) }}%</span
        >
        <div class="flex-1 min-w-0">
          <p class="truncate">
            <a :href="`https://musicbrainz.org/release/${candidate.id}`" rel="noopener" target="_blank">{{
              candidate.title
            }}</a>
            <span class="text-(--schemes-on-surface-variant)"> · {{ candidate.artist }}</span>
          </p>
          <p class="text-sm text-(--schemes-on-surface-variant)">
            {{ [candidate.date, candidate.country, candidate.format].filter(Boolean).join(' · ') }}
            · {{ pluralize(candidate.track_count, 'track') }}
          </p>
        </div>
        <M3Button @click.prevent="lookUp(candidate.id)">Use this</M3Button>
      </li>
    </ul>

    <form class="flex gap-2 items-start max-w-[640px]" @submit.prevent="handleSubmit">
      <M3TextField
        v-model="data.release"
        class="flex-1"
        label="MusicBrainz release link"
        name="release"
        placeholder="Paste a MusicBrainz release link"
        required
      />
      <M3Button class="mt-2" type="submit">Use it</M3Button>
    </form>

    <div>
      <M3Button variant="text" @click.prevent="lookUp()">Look it up again</M3Button>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { computed, watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import M3Button from '@/components/m3/M3Button.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{ album: Album }>()

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

// Under the album's key: editing the album makes it stale.
const { data: details, error } = useQuery({
  queryKey: computed(() => ['album', props.album.id, 'details']),
  queryFn: () => huntingService.albumDetails(props.album),
})
watch(error, error => error && handleHttpError(error))

const queued = () => toastSuccess('Looking it up. Jobs shows when it is done.')

const lookUp = async (release?: string) => {
  try {
    await huntingService.lookUpAlbum(props.album, release)
    queued()
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

const { data, handleSubmit } = useForm<{ release: string }>({
  initialValues: { release: '' },
  onSubmit: async ({ release }) => await huntingService.lookUpAlbum(props.album, release.trim()),
  onSuccess: () => {
    data.release = ''
    queued()
  },
})
</script>

<style scoped>
.release-link {
  color: var(--schemes-primary);
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>
