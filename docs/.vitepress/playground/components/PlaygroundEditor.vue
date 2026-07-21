<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, shallowRef } from 'vue'
import { useData } from 'vitepress'

const props = defineProps<{
  modelValue: string
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const container = ref<HTMLElement>()
const editorInstance = shallowRef<any>(null)
const { isDark } = useData()

onMounted(async () => {
  const monaco = await import('monaco-editor')
  const { dewLanguage, dewLanguageConfiguration } = await import('../languages/dew-monarch')

  monaco.languages.register({ id: 'dew' })
  monaco.languages.setMonarchTokensProvider('dew', dewLanguage)
  monaco.languages.setLanguageConfiguration('dew', dewLanguageConfiguration)

  if (!container.value) return

  const editor = monaco.editor.create(container.value, {
    value: props.modelValue,
    language: 'dew',
    theme: isDark.value ? 'vs-dark' : 'vs',
    minimap: { enabled: false },
    lineNumbers: 'off',
    glyphMargin: false,
    folding: false,
    scrollBeyondLastLine: false,
    renderLineHighlight: 'none',
    overviewRulerLanes: 0,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    scrollbar: {
      vertical: 'hidden',
      horizontal: 'auto',
    },
    padding: { top: 12, bottom: 12 },
    fontSize: 14,
    fontFamily: 'var(--vp-font-family-mono)',
    automaticLayout: true,
    wordWrap: 'on',
    placeholder: props.placeholder,
  })

  editor.onDidChangeModelContent(() => {
    const value = editor.getValue()
    if (value !== props.modelValue) {
      emit('update:modelValue', value)
    }
  })

  editorInstance.value = { editor, monaco }
})

watch(isDark, (dark) => {
  if (editorInstance.value) {
    editorInstance.value.monaco.editor.setTheme(dark ? 'vs-dark' : 'vs')
  }
})

watch(
  () => props.modelValue,
  (newVal) => {
    if (editorInstance.value) {
      const current = editorInstance.value.editor.getValue()
      if (current !== newVal) {
        editorInstance.value.editor.setValue(newVal)
      }
    }
  },
)

onBeforeUnmount(() => {
  if (editorInstance.value) {
    editorInstance.value.editor.dispose()
  }
})
</script>

<template>
  <div ref="container" class="playground-editor" />
</template>

<style scoped>
.playground-editor {
  width: 100%;
  min-height: 80px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  overflow: hidden;
}
</style>
