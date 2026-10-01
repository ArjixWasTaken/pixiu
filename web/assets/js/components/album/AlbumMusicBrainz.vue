<template>
  <section v-if="details" class="flex flex-col gap-4">
    <h3 class="text-xl">MusicBrainz</h3>

    <p v-if="details.enrichment === 'matched'">
      Matched to
      <a :href="`https://musicbrainz.org/release/${details.mbid}`" rel="noopener" target="_blank">a release</a
      >{{ details.enriched_at ? ` ${timeAgo(details.enriched_at)}` : '' }}; its tags follow it.
    </p>
    <p v-else-if="details.enrichment === 'review'" class="text-k-warning">
      píxiū isn’t sure which release this is. Pick one below, or paste a MusicBrainz release link.
    </p>
    <p v-else-if="details.enrichment === 'unmatched'">MusicBrainz knows nothing like it.</p>
    <p v-else class="text-k-fg-70">Not looked up yet.</p>

    <ul v-if="details.candidates.length" class="flex flex-col gap-2">
      <li
        v-for="candidate in details.candidates"
        :key="candidate.id"
        class="flex items-center gap-4 p-3 rounded-lg bg-k-fg-5"
      >
        <span class="w-12 text-center tabular-nums text-k-fg-70">{{ Math.round(candidate.score * 100) }}%</span>
        <div class="flex-1 min-w-0">
          <p class="truncate">
            <a :href="`https://musicbrainz.org/release/${candidate.id}`" rel="noopener" target="_blank">{{
              candidate.title
            }}</a>
            <span class="text-k-fg-70"> · {{ candidate.artist }}</span>
          </p>
          <p class="text-sm text-k-fg-50">
            {{ [candidate.date, candidate.country, candidate.format].filter(Boolean).join(' · ') }}
            · {{ pluralize(candidate.track_count, 'track') }}
          </p>
        </div>
        <Btn size="small" @click.prevent="lookUp(candidate.id)">Use this</Btn>
      </li>
    </ul>

    <form class="flex gap-2 max-w-[640px]" @submit.prevent="handleSubmit">
      <TextInput
        v-model="data.release"
        class="flex-1"
        name="release"
        placeholder="A MusicBrainz release link"
        required
      />
      <Btn type="submit">Use it</Btn>
    </form>

    <div>
      <Btn variant="ghost" @click.prevent="lookUp()">Look it up again</Btn>
    </div>
  </section>
</template>

<script lang="ts" setup>
import { onMounted, ref, watch } from 'vue'
import { huntingService } from '@/services/huntingService'
import type { AlbumDetails } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'
import { useForm } from '@/composables/useForm'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { useErrorHandler } from '@/composables/useErrorHandler'

import Btn from '@/components/ui/form/Btn.vue'
import TextInput from '@/components/ui/form/TextInput.vue'

const props = defineProps<{ album: Album }>()

const { toastSuccess } = useMessageToaster()
const { handleHttpError } = useErrorHandler('dialog')

const details = ref<AlbumDetails | null>(null)

const fetchDetails = async () => {
  try {
    details.value = await huntingService.albumDetails(props.album)
  } catch (error: unknown) {
    handleHttpError(error)
  }
}

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

onMounted(fetchDetails)
watch(() => props.album.id, fetchDetails)
</script>
