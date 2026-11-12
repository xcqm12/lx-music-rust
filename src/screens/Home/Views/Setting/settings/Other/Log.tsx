import { memo, useRef, useState, useEffect } from 'react'
import { View } from 'react-native'
import { getLogs, clearLogs } from '@/utils/log'
import { uploadNow, setErrorReportToken, getErrorReportToken } from '@/utils/errorReport'
// import { gzip, ungzip } from 'pako'

import SubTitle from '../../components/SubTitle'
import Button from '../../components/Button'
import { createStyle, toast } from '@/utils/tools'
import ConfirmAlert, { type ConfirmAlertType } from '@/components/common/ConfirmAlert'
import CheckBoxItem from '../../components/CheckBoxItem'
import { useI18n } from '@/lang'
import { updateSetting } from '@/core/common'
import { useSettingValue } from '@/store/setting/hook'
import Input from '@/components/common/Input'
import Text from '@/components/common/Text'

export default memo(() => {
  const t = useI18n()
  const alertRef = useRef<ConfirmAlertType>(null)
  const tokenAlertRef = useRef<ConfirmAlertType>(null)
  const [logText, setLogText] = useState('')
  const [tokenInput, setTokenInput] = useState('')
  const isUnmountedRef = useRef(true)
  const isAutoUpload = useSettingValue('common.isAutoUploadErrorLog')
  const [isEnableSyncErrorLog, setIsEnableSyncErrorLog] = useState(global.lx.isEnableSyncLog)
  const [isEnableUserApiLog, setIsEnableUserApiLog] = useState(global.lx.isEnableUserApiLog)

  const getErrorLog = () => {
    void getLogs().then(log => {
      if (isUnmountedRef.current || !log) return
      const logArr = log.split(/^----lx log----\n|\n----lx log----\n|\n----lx log----$/)
      // console.log(logArr)
      logArr.reverse()
      setLogText(logArr.join('\n\n').replace(/^\n+|\n+$/, ''))
    })
  }

  const openLogModal = () => {
    getErrorLog()
    alertRef.current?.setVisible(true)
  }

  const handleCleanLog = () => {
    void clearLogs().then(() => {
      toast(t('setting_other_log_tip_clean_success'))
      getErrorLog()
    })
  }

  const handleSetEnableSyncErrorLog = (enable: boolean) => {
    setIsEnableSyncErrorLog(enable)
    global.lx.isEnableSyncLog = enable
  }

  const handleSetEnableUserApiLog = (enable: boolean) => {
    setIsEnableUserApiLog(enable)
    global.lx.isEnableUserApiLog = enable
  }

  const handleSetAutoUpload = (enable: boolean) => {
    updateSetting({ 'common.isAutoUploadErrorLog': enable })
  }

  const handleUploadNow = () => {
    toast(t('setting_other_log_tip_uploading'))
    void uploadNow().then(({ ok, message }) => {
      toast(ok ? message : `${t('setting_other_log_tip_upload_fail')}: ${message}`)
    })
  }

  const openTokenModal = () => {
    void getErrorReportToken().then(value => {
      if (isUnmountedRef.current) return
      setTokenInput(value)
      tokenAlertRef.current?.setVisible(true)
    })
  }

  const handleSaveToken = () => {
    void setErrorReportToken(tokenInput).then(() => {
      toast(tokenInput.trim() ? t('setting_other_log_tip_token_saved') : t('setting_other_log_tip_token_cleared'))
    })
  }


  useEffect(() => {
    isUnmountedRef.current = false
    return () => {
      isUnmountedRef.current = true
    }
  }, [])

  return (
    <>
      <SubTitle title={t('setting_other_log')}>
        <View style={styles.checkBox}>
          <CheckBoxItem check={isAutoUpload} label={t('setting_other_log_auto_upload')} onChange={handleSetAutoUpload} />
          <CheckBoxItem check={isEnableSyncErrorLog} label={t('setting_other_log_sync_log')} onChange={handleSetEnableSyncErrorLog} />
          <CheckBoxItem check={isEnableUserApiLog} label={t('setting_other_log_user_api_log')} onChange={handleSetEnableUserApiLog} />
        </View>
        <View style={styles.btn}>
          <Button onPress={openLogModal}>{t('setting_other_log_btn_show')}</Button>
        </View>
        <View style={styles.btn}>
          <Button onPress={handleUploadNow}>{t('setting_other_log_btn_upload')}</Button>
          <Button onPress={openTokenModal}>{t('setting_other_log_btn_token')}</Button>
        </View>
        <Text size={12} style={styles.tip}>{t('setting_other_log_tip_upload')}</Text>
      </SubTitle>
      <ConfirmAlert
        ref={alertRef}
        cancelText={t('setting_other_log_btn_hide')}
        confirmText={t('setting_other_log_btn_clean')}
        onConfirm={handleCleanLog}
        showConfirm={!!logText}
        reverseBtn={true}
        >
        <View onStartShouldSetResponder={() => true}>
          {
            logText
              ? <Text selectable size={13}>{ logText }</Text>
              : <Text size={13}>{t('setting_other_log_tip_null')}</Text>
          }
        </View>
      </ConfirmAlert>
      <ConfirmAlert
        ref={tokenAlertRef}
        cancelText={t('setting_other_log_btn_hide')}
        confirmText={t('setting_other_log_btn_save')}
        onConfirm={handleSaveToken}
        >
        <Text size={13}>{t('setting_other_log_token_tip')}</Text>
        <Input
          style={styles.tokenInput}
          value={tokenInput}
          onChangeText={setTokenInput}
          autoCapitalize="none"
          autoCorrect={false}
          secureTextEntry
        />
      </ConfirmAlert>
    </>
  )
})

const styles = createStyle({
  checkBox: {
    // paddingTop: 10,
    paddingBottom: 15,
    marginLeft: -25,
  },
  btn: {
    flexDirection: 'row',
    gap: 10,
    marginBottom: 10,
  },
  tip: {
    opacity: 0.6,
  },
  tokenInput: {
    marginTop: 10,
    borderWidth: 1,
    borderRadius: 4,
    borderColor: 'rgba(128,128,128,0.4)',
  },
})
