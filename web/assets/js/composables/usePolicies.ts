import { useAuthorization } from '@/composables/useAuthorization'
import { Filter } from '@/config/hooks'
import { applyFilters } from '@/hooks'

export const usePolicies = () => {
  const { currentUser } = useAuthorization()

  const currentUserCan = {
    editSong: (_songs: MaybeArray<Song>) => currentUser.value.abilities.includes('manage songs'),
    editPlaylist: (playlist: Playlist) => playlist.permissions.edit,
    deletePlaylist: (playlist: Playlist) => playlist.permissions.delete,
    editAlbum: (album: Album) => album.permissions.edit,
    editArtist: (artist: Artist) => artist.permissions.edit,
    editUser: (user: User) => user.permissions.edit,
    deleteUser: (user: User) => user.permissions.delete,
    manageSettings: () => currentUser.value.abilities.includes('manage settings'),
    manageUsers: () => currentUser.value.abilities.includes('manage users'),
    uploadSongs: () => currentUser.value.abilities.includes('manage songs'),
  }

  return {
    currentUserCan: applyFilters(Filter.POLICIES, currentUserCan),
  }
}
