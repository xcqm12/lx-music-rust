/**
 * Custom Source Management Screen
 * Allows users to add, edit, and remove custom music source plugins
 */

import { useState, useEffect, useCallback } from 'react'
import { View, ScrollView, TouchableOpacity, TextInput, Alert, Platform } from 'react-native'
import { useTheme } from '@/store/theme/hook'
import { createStyle } from '@/utils/tools'
import Text from '@/components/common/Text'
import Button from '@/components/common/Button'
import Loading from '@/components/common/Loading'
import Modal from '@/components/common/Modal'
import { useI18n } from '@/lang'
import { rustBridge, type MusicInfo } from '@/utils/rust'

interface SourceItem {
  id: string
  name: string
  enabled: boolean
  code?: string
}

export default () => {
  const theme = useTheme()
  const t = useI18n()
  const [sources, setSources] = useState<SourceItem[]>([])
  const [loading, setLoading] = useState(false)
  const [engineReady, setEngineReady] = useState(false)
  const [modalVisible, setModalVisible] = useState(false)
  const [editorContent, setEditorContent] = useState('')
  const [editingSource, setEditingSource] = useState<SourceItem | null>(null)
  const [sourceName, setSourceName] = useState('')
  const [sourceId, setSourceId] = useState('')

  // Initialize Rust engine
  useEffect(() => {
    const initEngine = async () => {
      try {
        const result = await rustBridge.initEngine()
        setEngineReady(result)
        if (result) {
          loadSources()
        }
      } catch (error) {
        console.error('Failed to init engine:', error)
      }
    }
    initEngine()
  }, [])

  const loadSources = useCallback(async () => {
    setLoading(true)
    try {
      const loadedSources = await rustBridge.getSources()
      const items: SourceItem[] = loadedSources.map(([id, name]) => ({
        id,
        name,
        enabled: true,
      }))
      setSources(items)
    } catch (error) {
      console.error('Failed to load sources:', error)
    } finally {
      setLoading(false)
    }
  }, [])

  const handleAddSource = () => {
    setEditingSource(null)
    setSourceName('')
    setSourceId('')
    setEditorContent(getDefaultSourceTemplate())
    setModalVisible(true)
  }

  const handleEditSource = (source: SourceItem) => {
    setEditingSource(source)
    setSourceName(source.name)
    setSourceId(source.id)
    setEditorContent(source.code || getDefaultSourceTemplate())
    setModalVisible(true)
  }

  const handleDeleteSource = (source: SourceItem) => {
    Alert.alert(
      t('custom_source_delete_confirm', { name: source.name }),
      t('custom_source_delete_warning'),
      [
        { text: t('cancel'), style: 'cancel' },
        {
          text: t('confirm'),
          style: 'destructive',
          onPress: async () => {
            try {
              await rustBridge.removeSource(source.id)
              loadSources()
            } catch (error) {
              console.error('Failed to delete source:', error)
              Alert.alert(t('error'), String(error))
            }
          },
        },
      ],
    )
  }

  const handleSaveSource = async () => {
    if (!sourceName.trim()) {
      Alert.alert(t('error'), t('custom_source_name_required'))
      return
    }
    if (!sourceId.trim()) {
      Alert.alert(t('error'), t('custom_source_id_required'))
      return
    }

    setLoading(true)
    try {
      // Validate code first
      const isValid = await rustBridge.validateCode(editorContent)
      if (!isValid) {
        Alert.alert(t('error'), t('custom_source_invalid_code'))
        setLoading(false)
        return
      }

      // Load source
      await rustBridge.loadSource(sourceId, sourceName, editorContent)
      setModalVisible(false)
      loadSources()
    } catch (error) {
      console.error('Failed to save source:', error)
      Alert.alert(t('error'), String(error))
    } finally {
      setLoading(false)
    }
  }

  const handleTestSource = async () => {
    if (!editingSource) return

    setLoading(true)
    try {
      const results = await rustBridge.search(editingSource.id, '测试')
      Alert.alert(t('success'), t('custom_source_test_success', { count: results.length }))
    } catch (error) {
      console.error('Test failed:', error)
      Alert.alert(t('error'), String(error))
    } finally {
      setLoading(false)
    }
  }

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Text size={15} style={styles.title}>{t('custom_source_title')}</Text>
        <Button size='small' onPress={handleAddSource}>{t('custom_source_add')}</Button>
      </View>

      {!engineReady && (
        <View style={styles.notice}>
          <Text color={theme['c-primary-font']}>{t('custom_source_engine_not_ready')}</Text>
        </View>
      )}

      {loading && <Loading />}

      <ScrollView style={styles.list}>
        {sources.length === 0 && !loading && (
          <View style={styles.empty}>
            <Text color={theme['c-font']}>{t('custom_source_empty')}</Text>
            <Text size={12} color={theme['c-font-secondary']} style={styles.emptyHint}>
              {t('custom_source_empty_hint')}
            </Text>
          </View>
        )}

        {sources.map((source) => (
          <View key={source.id} style={[styles.item, { borderColor: theme['c-border-background'] }]}>
            <View style={styles.itemInfo}>
              <Text numberOfLines={1}>{source.name}</Text>
              <Text size={12} color={theme['c-font-secondary']} numberOfLines={1}>{source.id}</Text>
            </View>
            <View style={styles.itemActions}>
              <TouchableOpacity
                style={styles.actionBtn}
                onPress={() => handleEditSource(source)}
              >
                <Text size={13} color={theme['c-primary-font']}>{t('edit')}</Text>
              </TouchableOpacity>
              <TouchableOpacity
                style={styles.actionBtn}
                onPress={() => handleDeleteSource(source)}
              >
                <Text size={13} color={theme['c-danger-font']}>{t('delete')}</Text>
              </TouchableOpacity>
            </View>
          </View>
        ))}
      </ScrollView>

      {/* Editor Modal */}
      <Modal
        visible={modalVisible}
        onClose={() => setModalVisible(false)}
        title={editingSource ? t('custom_source_edit') : t('custom_source_add')}
      >
        <ScrollView style={styles.modalContent}>
          <View style={styles.formGroup}>
            <Text style={styles.label}>{t('custom_source_name')}</Text>
            <TextInput
              style={[styles.input, { backgroundColor: theme['c-content-background'], color: theme['c-font'] }]}
              value={sourceName}
              onChangeText={setSourceName}
              placeholder={t('custom_source_name_placeholder')}
              placeholderTextColor={theme['c-font-secondary']}
            />
          </View>

          <View style={styles.formGroup}>
            <Text style={styles.label}>{t('custom_source_id')}</Text>
            <TextInput
              style={[styles.input, { backgroundColor: theme['c-content-background'], color: theme['c-font'] }]}
              value={sourceId}
              onChangeText={setSourceId}
              placeholder={t('custom_source_id_placeholder')}
              placeholderTextColor={theme['c-font-secondary']}
              editable={!editingSource}
            />
          </View>

          <View style={styles.formGroup}>
            <Text style={styles.label}>{t('custom_source_code')}</Text>
            <TextInput
              style={[styles.codeInput, { backgroundColor: theme['c-content-background'], color: theme['c-font'] }]}
              value={editorContent}
              onChangeText={setEditorContent}
              multiline
              textAlignVertical="top"
              placeholder={t('custom_source_code_placeholder')}
              placeholderTextColor={theme['c-font-secondary']}
              autoCapitalize="none"
              autoCorrect={false}
            />
          </View>

          <View style={styles.modalActions}>
            {editingSource && (
              <Button size='small' onPress={handleTestSource} style={styles.testBtn}>
                {t('custom_source_test')}
              </Button>
            )}
            <Button size='small' type='primary' onPress={handleSaveSource}>
              {t('save')}
            </Button>
          </View>
        </ScrollView>
      </Modal>
    </View>
  )
}

function getDefaultSourceTemplate(): string {
  return `// Custom Music Source Plugin
// This module provides search and music info retrieval

module.exports = {
  // Search music by keyword
  search: async (keyword) => {
    // TODO: Implement your search logic
    // Return format: [{ id, name, singer, source, duration }]
    return []
  },

  // Get detailed music info
  getMusicInfo: async (musicId) => {
    // TODO: Implement music info retrieval
    // Return format: { id, name, singer, albumName, picUrl, ... }
    return null
  },

  // Get lyric by music id
  getLyric: async (musicId) => {
    // TODO: Implement lyric retrieval
    // Return format: { lyric: string, translation?: string }
    return null
  },

  // Get music URL by quality
  getUrl: async (musicId, quality) => {
    // TODO: Implement URL retrieval
    // Return format: { url: string }
    return null
  },

  // Get album picture
  getPic: async (musicId) => {
    // TODO: Implement picture retrieval
    // Return format: { url: string }
    return null
  }
}
`
}

const styles = createStyle({
  container: {
    flex: 1,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 15,
    paddingBottom: 10,
  },
  title: {
    fontWeight: 'bold',
  },
  notice: {
    padding: 10,
    marginHorizontal: 15,
    marginBottom: 10,
    borderRadius: 8,
    backgroundColor: 'rgba(0,0,0,0.1)',
  },
  list: {
    flex: 1,
  },
  empty: {
    padding: 30,
    alignItems: 'center',
  },
  emptyHint: {
    marginTop: 10,
    textAlign: 'center',
  },
  item: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 15,
    borderBottomWidth: 1,
  },
  itemInfo: {
    flex: 1,
    marginRight: 10,
  },
  itemActions: {
    flexDirection: 'row',
  },
  actionBtn: {
    paddingHorizontal: 10,
    paddingVertical: 5,
  },
  modalContent: {
    maxHeight: 400,
  },
  formGroup: {
    marginBottom: 15,
  },
  label: {
    marginBottom: 5,
    fontWeight: '500',
  },
  input: {
    borderRadius: 8,
    padding: 10,
    fontSize: 14,
  },
  codeInput: {
    borderRadius: 8,
    padding: 10,
    fontSize: 12,
    minHeight: 200,
    fontFamily: Platform.OS === 'ios' ? 'Menlo' : 'monospace',
  },
  modalActions: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    marginTop: 10,
    gap: 10,
  },
  testBtn: {
    marginRight: 'auto',
  },
})
