import { computed } from 'vue'
import { useUserStorage } from '@/composables/useUserStorage'
import { platforms } from '@/config/platforms'

/** The platforms Discover searches, in the order offered. */
export const searchablePlatforms = ['youtube_music', 'deezer']

/** The platform Discover searches, remembered for each user. */
export const useDiscoverPlatform = () => {
  const stored = useUserStorage<string>('discover-platform', searchablePlatforms[0])

  const platform = computed({
    get: () => (searchablePlatforms.includes(stored.value) ? stored.value : searchablePlatforms[0]),
    set: value => (stored.value = value),
  })

  const name = computed(() => platforms[platform.value].name)

  return { platform, name }
}
