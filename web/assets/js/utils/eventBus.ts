import mitt from 'mitt'
import type { Events } from '@/config/events'

export const eventBus = mitt<Events>()
