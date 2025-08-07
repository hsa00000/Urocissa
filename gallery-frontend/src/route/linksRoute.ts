// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LinksPage from '@/components/Page/LinksPage.vue'
import { useFilterStringStore } from '@/store/filterStringStore'

export const linksRoute: RouteRecordRaw = {
  path: '/links',
  component: LinksPage,
  name: 'links',

  meta: {
    isReadPage: false,
    isViewPage: false,
    filterString: null,
    baseName: 'links',
    getParentPage: (route) => {
      return {
        name: 'home',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route) => {
      return {
        name: 'links',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    beforeEnter: () => {
      const filterStringStore = useFilterStringStore()
      filterStringStore.filterString = null
    }
  }
}
