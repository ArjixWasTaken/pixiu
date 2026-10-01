import { toRef } from 'vue'
import { useUserStore } from '@/stores/userStore'
export const useAuthorization = () => {
  return {
    currentUser: toRef(useUserStore().state, 'current'),
  }
}
