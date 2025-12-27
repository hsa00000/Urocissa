// src/router.ts

import { RouteRecordRaw } from 'vue-router'
import 'vue-router'

import LinksPage from '@/components/Page/LinksPage.vue'

export const linksRoute: RouteRecordRaw = {
  path: '/links',
  component: LinksPage,
  name: 'links',
  meta: {
    basicString: null,
    level: 1,
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
    }
  }
}
