import { computed } from 'vue'
import { useConstStore } from '@/store/constStore'

export function useThemeClasses() {
  const constStore = useConstStore('mainId')

  const placeholderBgClass = computed(() => {
    const isLight = constStore.theme === 'light'
    return isLight ? 'bg-grey-lighten-2' : 'bg-grey-darken-2'
  })

  // You can add more theme-aware classes here
  const cardBgClass = computed(() => {
    const isLight = constStore.theme === 'light'
    return isLight ? 'bg-white' : 'bg-grey-darken-4'
  })

  const textClass = computed(() => {
    const isLight = constStore.theme === 'light'
    return isLight ? 'text-black' : 'text-white'
  })

  return {
    placeholderBgClass,
    cardBgClass,
    textClass
  }
}