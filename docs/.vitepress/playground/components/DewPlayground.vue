<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import PlaygroundEditor from './PlaygroundEditor.vue'
import AstViewer from './AstViewer.vue'
import OutputPanel from './OutputPanel.vue'

// Types matching WASM output
interface AstNode {
  type: string
  value?: string
  children?: AstNode[]
}

interface ParseResult {
  ok: boolean
  ast?: AstNode
  error?: string
}

interface CodeResult {
  ok: boolean
  code?: string
  error?: string
}

type Profile = 'scalar' | 'linalg' | 'complex' | 'quaternion'
type VarTypes = Record<string, string>

interface DewWasm {
  parse: (input: string) => ParseResult
  emit_wgsl: (input: string) => CodeResult
  emit_glsl: (input: string) => CodeResult
  emit_lua: (input: string) => CodeResult
  emit_wgsl_linalg?: (input: string, varTypes: VarTypes) => CodeResult
  emit_glsl_linalg?: (input: string, varTypes: VarTypes) => CodeResult
  emit_lua_linalg?: (input: string, varTypes: VarTypes) => CodeResult
  emit_wgsl_complex?: (input: string, varTypes: VarTypes) => CodeResult
  emit_glsl_complex?: (input: string, varTypes: VarTypes) => CodeResult
  emit_lua_complex?: (input: string, varTypes: VarTypes) => CodeResult
  emit_wgsl_quaternion?: (input: string, varTypes: VarTypes) => CodeResult
  emit_glsl_quaternion?: (input: string, varTypes: VarTypes) => CodeResult
  emit_lua_quaternion?: (input: string, varTypes: VarTypes) => CodeResult
}

const PROFILES = [
  { id: 'scalar' as Profile, label: 'Scalar', description: 'Basic math', exampleTypes: '' },
  { id: 'linalg' as Profile, label: 'Linalg', description: 'Vectors & matrices', exampleTypes: '{"v": "vec3", "m": "mat4"}' },
  { id: 'complex' as Profile, label: 'Complex', description: 'Complex numbers', exampleTypes: '{"z": "complex", "w": "complex"}' },
  { id: 'quaternion' as Profile, label: 'Quaternion', description: 'Rotations', exampleTypes: '{"q": "quat", "v": "vec3"}' },
]

const expression = ref('sin(x) + cos(y) * 2')
const profile = ref<Profile>('scalar')
const varTypesInput = ref('')
const activeTab = ref('AST')
const wasm = ref<DewWasm | null>(null)
const wasmLoading = ref(true)
const wasmError = ref(false)

const needsVarTypes = computed(() => profile.value !== 'scalar')

const varTypes = computed<VarTypes | null>(() => {
  if (!varTypesInput.value.trim()) return {}
  try {
    const parsed = JSON.parse(varTypesInput.value)
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) return null
    return parsed as VarTypes
  } catch {
    return null
  }
})

const varTypesError = computed(() => {
  if (!varTypesInput.value.trim()) return null
  return varTypes.value === null ? 'Invalid JSON' : null
})

const parseResult = computed<ParseResult>(() => {
  if (!expression.value.trim()) return { ok: false, error: 'Empty expression' }
  if (wasm.value) return wasm.value.parse(expression.value)
  return { ok: false, error: 'WASM not loaded' }
})

function emitCode(backend: 'wgsl' | 'glsl' | 'lua'): CodeResult {
  const w = wasm.value
  if (!w) return { ok: false, error: 'WASM not loaded' }
  const expr = expression.value
  const prof = profile.value
  const types = varTypes.value

  if (prof === 'scalar') {
    const fn = backend === 'wgsl' ? w.emit_wgsl : backend === 'glsl' ? w.emit_glsl : w.emit_lua
    return fn(expr)
  }

  if (types === null) return { ok: false, error: 'Invalid variable types JSON' }

  const fnMap: Record<string, Record<string, ((input: string, vt: VarTypes) => CodeResult) | undefined>> = {
    linalg: { wgsl: w.emit_wgsl_linalg, glsl: w.emit_glsl_linalg, lua: w.emit_lua_linalg },
    complex: { wgsl: w.emit_wgsl_complex, glsl: w.emit_glsl_complex, lua: w.emit_lua_complex },
    quaternion: { wgsl: w.emit_wgsl_quaternion, glsl: w.emit_glsl_quaternion, lua: w.emit_lua_quaternion },
  }

  const fn = fnMap[prof]?.[backend]
  if (!fn) return { ok: false, error: `${prof} backend not available` }
  return fn(expr, types)
}

const outputs = computed(() => ({
  AST: parseResult.value.ok ? { ok: true, code: 'ast' } : { ok: false, error: parseResult.value.error },
  WGSL: emitCode('wgsl'),
  GLSL: emitCode('glsl'),
  Lua: emitCode('lua'),
}))

function setProfile(p: Profile) {
  profile.value = p
  const meta = PROFILES.find((pr) => pr.id === p)
  if (meta?.exampleTypes && !varTypesInput.value) {
    varTypesInput.value = meta.exampleTypes
  }
}

onMounted(async () => {
  try {
    // In the docs build, WASM files are served from /dew/wasm/
    const wasmModule = await import(/* @vite-ignore */ `${import.meta.env.BASE_URL}wasm/dew_wasm.js`)
    await wasmModule.default()
    wasm.value = {
      parse: wasmModule.parse,
      emit_wgsl: wasmModule.emit_wgsl,
      emit_glsl: wasmModule.emit_glsl,
      emit_lua: wasmModule.emit_lua,
      emit_wgsl_linalg: wasmModule.emit_wgsl_linalg,
      emit_glsl_linalg: wasmModule.emit_glsl_linalg,
      emit_lua_linalg: wasmModule.emit_lua_linalg,
      emit_wgsl_complex: wasmModule.emit_wgsl_complex,
      emit_glsl_complex: wasmModule.emit_glsl_complex,
      emit_lua_complex: wasmModule.emit_lua_complex,
      emit_wgsl_quaternion: wasmModule.emit_wgsl_quaternion,
      emit_glsl_quaternion: wasmModule.emit_glsl_quaternion,
      emit_lua_quaternion: wasmModule.emit_lua_quaternion,
    }
  } catch (e) {
    console.warn('WASM not available:', e)
    wasmError.value = true
  } finally {
    wasmLoading.value = false
  }
})
</script>

<template>
  <div class="dew-playground">
    <div class="dew-playground__toolbar">
      <div class="dew-playground__profiles">
        <button
          v-for="p in PROFILES"
          :key="p.id"
          class="profile-btn"
          :class="{ 'profile-btn--active': p.id === profile }"
          @click="setProfile(p.id)"
        >
          {{ p.label }}
        </button>
      </div>
      <div class="dew-playground__status">
        <span v-if="wasmLoading" class="status status--loading">Loading WASM...</span>
        <span v-else-if="wasmError" class="status status--error">WASM unavailable</span>
        <span v-else class="status status--ok">Ready</span>
      </div>
    </div>

    <div class="dew-playground__grid">
      <div class="dew-playground__input">
        <div class="section-label">Expression</div>
        <PlaygroundEditor v-model="expression" placeholder="Enter a dew expression..." />
        <div v-if="needsVarTypes" class="var-types">
          <label class="var-types__label">
            Variable Types
            <span v-if="varTypesError" class="var-types__error">{{ varTypesError }}</span>
          </label>
          <input
            type="text"
            class="var-types__input"
            :class="{ 'var-types__input--error': !!varTypesError }"
            :value="varTypesInput"
            @input="varTypesInput = ($event.target as HTMLInputElement).value"
            :placeholder="PROFILES.find((p) => p.id === profile)?.exampleTypes"
          />
        </div>
      </div>

      <div class="dew-playground__output">
        <div class="section-label">Output</div>
        <div class="output-container">
          <template v-if="activeTab === 'AST'">
            <div class="output-content">
              <AstViewer v-if="parseResult.ok && parseResult.ast" :ast="parseResult.ast" />
              <div v-else class="output-error">{{ parseResult.error }}</div>
            </div>
          </template>
          <template v-else>
            <OutputPanel
              :tabs="['WGSL', 'GLSL', 'Lua']"
              :active-tab="activeTab"
              :outputs="outputs"
              @update:active-tab="activeTab = $event"
            />
          </template>

          <div class="tab-bar">
            <button
              v-for="tab in ['AST', 'WGSL', 'GLSL', 'Lua']"
              :key="tab"
              class="tab-btn"
              :class="{ 'tab-btn--active': tab === activeTab }"
              @click="activeTab = tab"
            >
              {{ tab }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dew-playground {
  margin: 24px 0;
}

.dew-playground__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  gap: 12px;
  flex-wrap: wrap;
}

.dew-playground__profiles {
  display: flex;
  gap: 4px;
}

.profile-btn {
  padding: 5px 12px;
  font-size: 13px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-2);
  cursor: pointer;
  transition: all 0.2s;
}

.profile-btn:hover {
  color: var(--vp-c-text-1);
  border-color: var(--vp-c-brand-1);
}

.profile-btn--active {
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
  border-color: var(--vp-c-brand-1);
}

.status {
  font-size: 12px;
  font-family: var(--vp-font-family-mono);
}

.status--loading {
  color: var(--vp-c-text-3);
}

.status--error {
  color: var(--vp-c-danger-1);
}

.status--ok {
  color: var(--vp-c-green-1);
}

.dew-playground__grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

@media (max-width: 768px) {
  .dew-playground__grid {
    grid-template-columns: 1fr;
  }
}

.section-label {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--vp-c-text-3);
  margin-bottom: 8px;
}

.var-types {
  margin-top: 10px;
}

.var-types__label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--vp-c-text-2);
  margin-bottom: 4px;
}

.var-types__error {
  color: var(--vp-c-danger-1);
  font-size: 11px;
}

.var-types__input {
  width: 100%;
  padding: 6px 10px;
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
}

.var-types__input:focus {
  outline: none;
  border-color: var(--vp-c-brand-1);
}

.var-types__input--error {
  border-color: var(--vp-c-danger-1);
}

.output-container {
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  overflow: hidden;
  background: var(--vp-c-bg);
  display: flex;
  flex-direction: column;
  min-height: 200px;
}

.output-content {
  flex: 1;
  padding: 12px;
  overflow: auto;
}

.output-error {
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  color: var(--vp-c-danger-1);
}

.tab-bar {
  display: flex;
  gap: 2px;
  padding: 6px 8px;
  border-top: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg-soft);
}

.tab-btn {
  padding: 4px 10px;
  font-size: 12px;
  font-family: var(--vp-font-family-mono);
  background: none;
  border: none;
  border-radius: 4px;
  color: var(--vp-c-text-3);
  cursor: pointer;
  transition: all 0.2s;
}

.tab-btn:hover {
  color: var(--vp-c-text-1);
}

.tab-btn--active {
  background: var(--vp-c-bg);
  color: var(--vp-c-brand-1);
}
</style>
