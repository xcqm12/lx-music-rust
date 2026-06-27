import { View } from 'react-native'
import { useTheme } from '@/store/theme/hook'
import { createStyle } from '@/utils/tools'
import Text from '@/components/common/Text'
// import { useWindowSize } from '@/utils/hooks'
const HEADER_HEIGHT = 20

interface Props {
  children: React.ReactNode
}

export default ({ children }: Props) => {
  const theme = useTheme()

  const memoChildren = (() => children)()
  const renderChildren = ((): any => {
    if (typeof memoChildren === 'string' || typeof memoChildren === 'number') {
      return <Text>{memoChildren}</Text>
    }
    return memoChildren
  })()

  return (
    <View style={{ ...styles.centeredView, backgroundColor: 'rgba(50,50,50,.3)' }}>
      <View style={{ ...styles.modalView, backgroundColor: theme['c-content-background'] }}>
        <View style={{ ...styles.header, backgroundColor: theme['c-primary-light-100-alpha-100'] }}></View>
        {renderChildren}
      </View>
    </View>
  )
}


const styles = createStyle({
  centeredView: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  modalView: {
    maxWidth: '90%',
    minWidth: '60%',
    maxHeight: '78%',
    // backgroundColor: 'white',
    borderRadius: 4,
    // shadowColor: '#000',
    // shadowOffset: {
    //   width: 0,
    //   height: 2,
    // },
    // shadowOpacity: 0.25,
    // shadowRadius: 4,
    elevation: 3,
  },
  header: {
    flexGrow: 0,
    flexShrink: 0,
    flexDirection: 'row',
    borderTopLeftRadius: 4,
    borderTopRightRadius: 4,
    height: HEADER_HEIGHT,
  },
})
