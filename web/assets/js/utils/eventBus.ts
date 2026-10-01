import { EventEmitter } from 'events'
import type { TypedEmitter } from 'tiny-typed-emitter'
import type { Events } from '@/config/events'

// tiny-typed-emitter only types Node's emitter; the emitter itself comes from
// `events`, which the bundle can always resolve (tiny-typed-emitter doesn't
// declare it, so it can't count on finding it).
const eventBus = new EventEmitter() as TypedEmitter<Events>
eventBus.setMaxListeners(100)

export { eventBus }
