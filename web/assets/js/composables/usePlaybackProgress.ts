import { computed, onBeforeUnmount, ref } from 'vue'
import { playback } from '@/services/playbackManager'
import { crossfadeService } from '@/services/crossfadeService'
import { useQueueStore } from '@/stores/queueStore'
import { secondsToHis } from '@/utils/formatters'

const activeMedia = (): HTMLMediaElement | null => {
  if (crossfadeService.active && crossfadeService.state) {
    return crossfadeService.state.incomingAudio
  }

  return playback('current')?.media ?? null
}

/** Where the current song is, polled from the playing media element. */
export const usePlaybackProgress = () => {
  const currentTime = ref(0)
  const duration = ref(0)
  const buffered = ref(0)
  const loading = ref(false)
  const seeking = ref(false)

  const update = () => {
    if (seeking.value) {
      return
    }

    const pendingStart = playback('current')?.pendingStart ?? null

    if (pendingStart !== null) {
      // A song restored from the saved queue, not started yet: where it starts, and its own length.
      currentTime.value = pendingStart
      duration.value = useQueueStore().current?.length ?? 0
      buffered.value = 0
      loading.value = false
      return
    }

    const media = activeMedia()

    if (!media) {
      return
    }

    currentTime.value = media.currentTime
    duration.value = Number.isFinite(media.duration) ? media.duration : 0
    buffered.value = media.buffered.length > 0 ? media.buffered.end(media.buffered.length - 1) : 0
    loading.value = !!media.src && media.readyState < 3 && media.currentTime === 0
  }

  const seek = (seconds: number) => {
    const service = playback('current')

    // Before a restored song starts, a seek sets where it starts.
    if (!service || (service.pendingStart === null && !service.media?.duration)) {
      return
    }

    currentTime.value = seconds
    service.seekTo(seconds)
  }

  const interval = setInterval(update, 250)
  onBeforeUnmount(() => clearInterval(interval))

  const percent = computed(() => (duration.value > 0 ? (currentTime.value / duration.value) * 100 : 0))
  const current = computed(() => secondsToHis(currentTime.value))
  // Before the media loads, the song's own length.
  const total = computed(() => secondsToHis(duration.value || useQueueStore().current?.length || 0))

  return { currentTime, duration, buffered, loading, seeking, percent, current, total, seek }
}
