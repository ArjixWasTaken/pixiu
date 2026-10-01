import { toRef } from 'vue'
import { commonStore } from '@/stores/commonStore'

export const useThirdPartyServices = () => ({
  useMusicBrainz: toRef(commonStore.state, 'uses_musicbrainz'),
})
