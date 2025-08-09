import { RouteRecordRaw } from 'vue-router'
import 'vue-router'
import ViewPageMain from '@/components/View/ViewPageMain.vue'

const albumRoute: RouteRecordRaw = {
  path: `/album-:albumId`,
  name: 'album',
  meta: {
    isReadPage: false,
    isViewPage: false,
    baseName: 'album',
    getParentPage: (route) => {
      return {
        name: 'album',
        params: { hash: undefined, subhash: undefined },
        query: route.query
      }
    },
    getChildPage: (route, hash) => {
      return {
        name: 'albumViewPage',
        params: { hash: hash, subhash: undefined },
        query: route.query
      }
    }
  },
  // No beforeEnter needed; filter string is computed locally in App.vue based on route meta.
  children: [
    {
      path: '/view/:hash',
      component: ViewPageMain,
      name: 'albumViewPage',
      meta: {
        isReadPage: false,
        isViewPage: true,
        baseName: 'album',
        getParentPage: (route) => {
          return {
            name: 'album',
            params: { hash: undefined, subhash: undefined },
            query: route.query
          }
        },
        getChildPage: undefined
      }
    }
  ]
}

export const albumRoutes: RouteRecordRaw[] = [albumRoute]
