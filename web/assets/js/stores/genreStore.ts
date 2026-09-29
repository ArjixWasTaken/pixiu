import { http } from '@/services/http'

export const genreStore = {
  fetchAll: async () => await http.get<Genre[]>('genres'),

  async fetchOne(id: Genre['id']) {
    const genre = (await this.fetchAll()).find(genre => genre.id === id)

    if (!genre) {
      throw new Error(`No genre ${id}`)
    }

    return genre
  },
}
