import { RouteRecordRaw } from 'vue-router'
import 'vue-router'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import { useFilterStringStore } from '@/store/filterStringStore'
import { virtualRouteNames, VirtualRouteName } from './pageReturnType'

function createVirtualRoute(baseName: VirtualRouteName): RouteRecordRaw {
  return {
    path: `/${baseName}`,
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
    beforeEnter: (_to, _from, next) => {
      const filterStringStore = useFilterStringStore()
      switch (baseName) {
        case 'home':
          filterStringStore.filterString = 'and(not(tag:"_archived"), not(tag:"_trashed"))'
          break
        case 'all':
          filterStringStore.filterString = 'not(tag:"_trashed")'
          break
        case 'favorite':
          filterStringStore.filterString = 'and(tag:"_favorite", not(tag:"_trashed"))'
          break
        case 'archived':
          filterStringStore.filterString = 'and(tag:"_archived", not(tag:"_trashed"))'
          break
        case 'trashed':
          filterStringStore.filterString = 'and(tag:"_trashed")'
          break
        case 'albums':
          filterStringStore.filterString = 'and(type:"album", not(tag:"_trashed"))'
          break
        case 'videos':
          filterStringStore.filterString =
            'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
          break
      }

      next()
    },
    children: [
      {
        path: '/view/:hash',
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
