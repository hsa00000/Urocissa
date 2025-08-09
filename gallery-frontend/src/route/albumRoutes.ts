import { RouteRecordRaw } from 'vue-router'
import 'vue-router'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import AlbumPage from '@/components/Page/AlbumPage.vue'

const albumRoute: RouteRecordRaw = {
  path: `/album-:albumId`,
  component: AlbumPage,
  name: 'album',
  props: true,
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
      path: ':hash',
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
