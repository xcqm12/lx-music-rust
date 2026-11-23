import { useState, forwardRef, useImperativeHandle, type Ref } from 'react'
import { FlatList, type FlatListProps } from 'react-native'

// import InsetShadow from 'react-native-inset-shadow'

export type ItemT<T> = FlatListProps<T>['data']

export type ListProps<T> = Pick<FlatListProps<T>,
| 'renderItem'
| 'maxToRenderPerBatch'
| 'windowSize'
| 'initialNumToRender'
| 'keyExtractor'
| 'getItemLayout'
| 'keyboardShouldPersistTaps'
>

export interface ListType<T> {
  setList: (list: T[]) => void
}

const List = <T extends ItemT<T>>(props: ListProps<T>, ref: Ref<ListType<T>>) => {
  const [list, setList] = useState<T[]>([])
  useImperativeHandle(ref, () => ({
    setList(list) {
      setList(list)
    },
  }))

  // RN Android 的 removeClippedSubviews 在打字联想词高频 setList 替换时易触发原生崩溃
  // （车机实测搜索框输入阶段即闪退），列表项极轻，关闭裁剪不影响内存。
  return <FlatList removeClippedSubviews={false} keyboardShouldPersistTaps={'always'} {...props} data={list} />
}

export default forwardRef(List) as
  <M,>(p: ListProps<M> & { ref?: Ref<ListType<M>> }) => JSX.Element | null


