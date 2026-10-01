import { toRef } from 'vue'
import { useCommonStore } from '@/stores/commonStore'
export const useThirdPartyServices = () => ({
  useMusicBrainz: toRef(useCommonStore().state, 'uses_musicbrainz'),
})
