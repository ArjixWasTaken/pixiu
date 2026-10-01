import { StorageSerializers, useStorage } from '@vueuse/core'
import type { MaybeRefOrGetter } from 'vue'
import { toValue } from 'vue'
import { useAuthorization } from '@/composables/useAuthorization'

/**
 * A value kept in localStorage, as a ref. Stored as JSON, as it always was,
 * so what browsers saved before still reads back; a default isn't written.
 * Writes happen at once: a sign-in token is often followed by a reload.
 */
export const useAppStorage = <T>(key: MaybeRefOrGetter<string>, initial: T) =>
  useStorage<T>(key, initial, localStorage, {
    serializer: StorageSerializers.object,
    writeDefaults: false,
    flush: 'sync',
  })

/** The same, kept apart for each signed-in user (`<user id>::<key>`): sort orders, the sidebar, columns. */
export const useUserStorage = <T>(key: MaybeRefOrGetter<string>, initial: T) => {
  const { currentUser } = useAuthorization()

  return useAppStorage<T>(() => `${currentUser.value?.id}::${toValue(key)}`, initial)
}
