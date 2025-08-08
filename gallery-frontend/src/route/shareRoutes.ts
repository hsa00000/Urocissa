import { RouteRecordRaw } from 'vue-router'
import 'vue-router'
import UniversalSharePage from '@/components/Page/UniversalSharePage.vue'
import ViewPageMain from '@/components/View/ViewPageMain.vue'
import { useShareStore } from '@/store/shareStore'
import { PageReturnType } from './pageReturnType'

export const shareRoute: RouteRecordRaw[] = [
  {
    path: '/share/:albumId-:shareId',
    component: UniversalSharePage,
    name: 'share',
    meta: {
      isReadPage: false,
      isViewPage: false,
      filterString: null,
      baseName: 'share',
      getParentPage: (route) => {
        return {
          name: 'share',
          params: { hash: undefined, subhash: undefined },
          query: route.query
        }
      },
      getChildPage: (route, hash) => {
        return {
          name: `shareViewPage`,
          params: { hash: hash, subhash: undefined },
          query: route.query
        }
      }
    },
  beforeEnter: (to, _from, next) => {
      const albumIdOpt = to.params.albumId
      const shareIdOpt = to.params.shareId

      // Only allow route if both albumId and shareId are strings
      if (typeof albumIdOpt === 'string' && typeof shareIdOpt === 'string') {
        // Set up store data
        const shareStore = useShareStore('mainId')
        shareStore.albumId = albumIdOpt
        shareStore.shareId = shareIdOpt

        next()
      } else {
        console.error(`(albumId, shareId) is (${String(albumIdOpt)}, ${String(shareIdOpt)})`)
        // Redirect to home page
        next('/home')
      }
    },
    children: [
      {
        path: 'view/:hash',
        component: ViewPageMain,
        name: `shareViewPage`,
        meta: {
          isReadPage: false,
          isViewPage: true,
          baseName: 'share',
          getParentPage: (route, albumId, shareId) => {
            return {
              name: 'share',
              params: { albumId: albumId, shareId: shareId },
              query: route.query
            }
          },
          getChildPage: function (): PageReturnType {
            throw new Error('Function not implemented.')
          }
        }
      }
    ]
  }
]
