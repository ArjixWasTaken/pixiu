const models: SmartPlaylistModel[] = [
  {
    name: 'title',
    type: 'text',
    label: 'Title',
  },
  {
    name: 'album.name',
    type: 'text',
    label: 'Album',
  },
  {
    name: 'artist.name',
    type: 'text',
    label: 'Artist',
  },
  {
    name: 'genre',
    type: 'text',
    label: 'Genre',
  },
  {
    name: 'year',
    type: 'number',
    label: 'Year',
  },
  {
    name: 'interactions.play_count',
    type: 'number',
    label: 'Play Count',
  },
  {
    name: 'interactions.last_played_at',
    type: 'date',
    label: 'Last Played',
  },
  {
    name: 'length',
    type: 'number',
    label: 'Length',
    unit: 'seconds',
  },
  {
    name: 'rating',
    type: 'number',
    label: 'Rating',
  },
  {
    name: 'created_at',
    type: 'date',
    label: 'Date Added',
  },
]

export default models
