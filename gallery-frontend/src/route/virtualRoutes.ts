import { RouteRecordRaw } from 'vue-router'
import 'vue-router'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import { virtualRouteNames, VirtualRouteName } from './pageReturnType'
import HomePage from '@/components/Page/HomePage.vue'

function createVirtualRoute(baseName: VirtualRouteName): RouteRecordRaw {
  return {
    path: `/${baseName}`,
    component: HomePage,
    name: baseName,
    meta: {
      isReadPage: false,
      isViewPage: false,
      baseName: baseName,
      getParentPage: (route) => {
        return {
          name: baseName,
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route, hash) => {
        return {
          name: `${baseName}ViewPage`,
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
        name: `${baseName}ViewPage`,
        meta: {
          isReadPage: false,
          isViewPage: true,
          baseName: baseName,
          getParentPage: (route) => {
            return {
              name: baseName,
              params: { hash: undefined, subhash: undefined },
              query: route.query
            }
          },
          getChildPage: undefined
        }
      }
    ]
  }
}

export const virtualRoutes: RouteRecordRaw[] = [...virtualRouteNames].map((baseName) =>
  createVirtualRoute(baseName)
)
