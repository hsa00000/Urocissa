import {
  computed,
  inject,
  onScopeDispose,
  readonly,
  shallowRef,
  watch,
  type ComputedRef,
  type InjectionKey,
  type Ref
} from 'vue'

export interface ScrollActivityContext {
  readonly active: Readonly<Ref<boolean>>
  readonly generation: Readonly<Ref<number>>
}

export const scrollActivityKey: InjectionKey<ScrollActivityContext> = Symbol('scrollActivity')

export function useScrollActivity(
  scrollTop: () => number,
  idleDelay = 100
): ScrollActivityContext {
  const active = shallowRef(false)
  const generation = shallowRef(0)
  let idleTimer: ReturnType<typeof setTimeout> | null = null

  watch(scrollTop, () => {
    generation.value += 1
    active.value = true

    if (idleTimer !== null) {
      clearTimeout(idleTimer)
    }

    idleTimer = setTimeout(() => {
      active.value = false
      idleTimer = null
    }, idleDelay)
  })

  onScopeDispose(() => {
    if (idleTimer !== null) {
      clearTimeout(idleTimer)
    }
  })

  return {
    active: readonly(active),
    generation: readonly(generation)
  }
}

export function useRowScrollActivity(
  providedActivity?: ScrollActivityContext
): ComputedRef<boolean> {
  const activity = providedActivity ?? inject(scrollActivityKey)
  if (activity === undefined) {
    throw new Error('scroll activity context was not provided')
  }

  const mountedGeneration = activity.generation.value
  return computed(
    () => activity.active.value && activity.generation.value > mountedGeneration
  )
}
