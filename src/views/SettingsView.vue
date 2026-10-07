<template>
  <div>
    <h1 class="page-title">设置</h1>
    <p class="page-subtitle">全局偏好、镜像源、源码默认值与初始化选项。</p>

    <InfoBar
      v-if="message"
      :IsOpen="true"
      :Severity="severity"
      :Title="severity === 'Error' ? '操作失败' : '完成'"
      :Message="message"
      :IsClosable="true"
      @CloseButtonClick="clear"
    />

    <!-- 外观 -->
    <div class="card">
      <div class="section-title">外观</div>
      <div class="row">
        <ComboBox
          :ItemsSource="themeOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selTheme"
          Header="主题"
          Width="200"
        />
        <TextBox
          v-model:Text="maxLines"
          Header="最大日志行数"
          PlaceholderText="5000"
          Width="140"
        />
      </div>
    </div>

    <!-- 工作区 -->
    <div class="card section">
      <div class="section-title">工作区</div>
      <div class="row">
        <TextBox
          v-model:Text="workspaceDir"
          Header="工作区目录"
          PlaceholderText="~/KernelBuilder"
          Width="400"
        />
        <Button Content="重新初始化" @Click="doBootstrap" />
        <Button Content="检查依赖" @Click="checkDeps" />
      </div>
      <p v-if="deps.length" class="muted" style="margin-top: 10px">
        <span
          v-for="d in deps"
          :key="d.name"
          class="badge"
          :class="d.present ? 'badge-ok' : d.required ? 'badge-err' : 'badge-mute'"
        >
          {{ d.name }}
        </span>
      </p>
    </div>

    <!-- 镜像源 -->
    <div class="card section">
      <div class="section-title">GitHub 镜像源</div>
      <div class="row">
        <ToggleSwitch v-model:IsOn="mirrorEnabled" Header="启用镜像" OnContent="开" OffContent="关" />
        <ToggleSwitch v-model:IsOn="autoFallback" Header="自动回退" OnContent="开" OffContent="关" />
        <ComboBox
          :ItemsSource="mirrorOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selMirror"
          Header="当前镜像"
          Width="260"
        />
        <Button Content="测试当前镜像" @Click="doTestMirror" />
      </div>
      <table class="kv-table" style="margin-top: 12px">
        <thead>
          <tr><th>ID</th><th>标签</th><th>类型</th><th>前缀</th><th>启用</th></tr>
        </thead>
        <tbody>
          <tr v-for="m in mirrors" :key="m.id">
            <td class="mono">{{ m.id }}</td>
            <td>{{ m.label }}</td>
            <td>{{ m.kind }}</td>
            <td class="mono muted">{{ m.prefix }}</td>
            <td>
              <ToggleSwitch
                :IsOn="m.enabled"
                @update:IsOn="(v: boolean) => toggleMirror(m.id, v)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 源码默认 -->
    <div class="card section">
      <div class="section-title">源码拉取默认值</div>
      <div class="row">
        <TextBox v-model:Text="sourceRepo" Header="默认仓库" Width="360" />
        <TextBox v-model:Text="sourceBranch" Header="默认分支" Width="160" />
        <ComboBox
          :ItemsSource="protocolOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selProtocol"
          Header="协议"
          Width="220"
        />
      </div>
      <div class="row" style="margin-top: 10px">
        <ToggleSwitch v-model:IsOn="shallow" Header="浅克隆" OnContent="--depth=1" OffContent="完整历史" />
        <TextBox v-if="shallow" v-model:Text="depth" Header="深度" Width="100" />
        <ToggleSwitch v-model:IsOn="singleBranch" Header="单分支" OnContent="是" OffContent="否" />
        <ToggleSwitch v-model:IsOn="submodules" Header="子模块" OnContent="递归" OffContent="不拉取" />
      </div>
    </div>

    <!-- GitHub 默认 -->
    <div class="card section">
      <div class="section-title">GitHub Actions 默认</div>
      <div class="row">
        <TextBox v-model:Text="ghToken" Header="Token" PlaceholderText="ghp_xxx" Width="340" />
        <TextBox v-model:Text="ghOwner" Header="Owner" Width="180" />
        <TextBox v-model:Text="ghRepo" Header="Repo" Width="220" />
        <TextBox v-model:Text="ghWorkflowId" Header="Workflow ID" Width="180" />
        <TextBox v-model:Text="ghRef" Header="Git Ref" Width="160" />
      </div>
    </div>

    <!-- 保存与重置 -->
    <div class="card section">
      <div class="row">
        <Button Content="保存全部设置" @Click="saveAll" />
        <Button Content="重置为默认" @Click="doReset" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

import { useTask } from '@/composables/useTask'
import { bootstrapNow, hostDependencies, resetConfig, testMirror } from '@/api/tauri'
import { applyTheme, config, loadConfig, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

/* ---------- 外观 ---------- */
const themeOptions = [
  { Tag: 'dark', Content: '深色' },
  { Tag: 'light', Content: '浅色' },
  { Tag: 'system', Content: '跟随系统' },
]
const selTheme = ref(themeOptions[0])
const maxLines = ref('5000')

/* ---------- 工作区 ---------- */
const workspaceDir = ref('')
const deps = ref<T.HostDependency[]>([])

/* ---------- 镜像 ---------- */
const mirrorEnabled = ref(true)
const autoFallback = ref(true)
const selMirror = ref<{ Tag: string; Content: string } | null>(null)
const mirrors = ref<T.Mirror[]>([])

const mirrorOptions = computed(() =>
  mirrors.value
    .filter((m) => m.enabled)
    .map((m) => ({ Tag: m.id, Content: `${m.label} · ${m.kind}` })),
)

/* ---------- 源码默认 ---------- */
const sourceRepo = ref('')
const sourceBranch = ref('main')
const protocolOptions = [
  { Tag: 'auto', Content: '自动' },
  { Tag: 'https', Content: 'HTTPS' },
  { Tag: 'ssh', Content: 'SSH' },
]
const selProtocol = ref(protocolOptions[0])
const shallow = ref(true)
const depth = ref('1')
const singleBranch = ref(true)
const submodules = ref(false)

/* ---------- GitHub ---------- */
const ghToken = ref('')
const ghOwner = ref('')
const ghRepo = ref('')
const ghWorkflowId = ref('')
const ghRef = ref('main')

/* ---------- 初始化 ---------- */
async function doBootstrap() {
  await run(async () => {
    const r = await bootstrapNow(true)
    if (r.ok) ok('初始化完成')
    else fail(`初始化未完成：${r.failed.join('、') || '依赖缺失'}`)
  })
}

async function checkDeps() {
  await run(async () => {
    deps.value = await hostDependencies()
  })
}

async function doTestMirror() {
  const id = selMirror.value?.Tag
  if (!id) return fail('请先选择一个镜像')
  await run(async () => {
    const r = await testMirror(id)
    if (r.ok) ok(r.message)
    else fail(r.message)
  })
}

async function toggleMirror(id: string, enabled: boolean) {
  const m = mirrors.value.find((x) => x.id === id)
  if (m) m.enabled = enabled
  if (!config.value) return
  config.value.mirror.mirrors = [...mirrors.value]
  await persist()
}

async function saveAll() {
  if (!config.value) return
  config.value.ui.theme = selTheme.value.Tag as 'light' | 'dark' | 'system'
  config.value.ui.maxLogLines = Number(maxLines.value) || 5000
  config.value.general.workspaceDir = workspaceDir.value
  config.value.mirror.enabled = mirrorEnabled.value
  config.value.mirror.autoFallback = autoFallback.value
  config.value.mirror.activeId = selMirror.value?.Tag ?? config.value.mirror.activeId
  config.value.source.repo = sourceRepo.value
  config.value.source.branch = sourceBranch.value
  config.value.source.protocol = selProtocol.value.Tag
  config.value.source.shallow = shallow.value
  config.value.source.depth = Number(depth.value) || 1
  config.value.source.singleBranch = singleBranch.value
  config.value.source.submodules = submodules.value
  config.value.github.token = ghToken.value
  config.value.github.owner = ghOwner.value
  config.value.github.repo = ghRepo.value
  config.value.github.workflowId = ghWorkflowId.value
  config.value.github.gitRef = ghRef.value
  await persist()
  applyTheme(config.value.ui.theme)
  ok('设置已保存')
}

async function doReset() {
  await run(async () => {
    await resetConfig()
    await loadConfig()
    syncFromConfig()
  }, '已恢复默认设置')
}

function syncFromConfig() {
  const c = config.value
  if (!c) return
  selTheme.value = themeOptions.find((t) => t.Tag === c.ui.theme) ?? themeOptions[0]
  maxLines.value = String(c.ui.maxLogLines)
  workspaceDir.value = c.general.workspaceDir
  mirrorEnabled.value = c.mirror.enabled
  autoFallback.value = c.mirror.autoFallback
  mirrors.value = c.mirror.mirrors
  selMirror.value = mirrorOptions.value.find((m) => m.Tag === c.mirror.activeId) ?? mirrorOptions.value[0] ?? null
  sourceRepo.value = c.source.repo
  sourceBranch.value = c.source.branch
  selProtocol.value = protocolOptions.find((p) => p.Tag === c.source.protocol) ?? protocolOptions[0]
  shallow.value = c.source.shallow
  depth.value = String(c.source.depth || 1)
  singleBranch.value = c.source.singleBranch
  submodules.value = c.source.submodules
  ghToken.value = c.github.token
  ghOwner.value = c.github.owner
  ghRepo.value = c.github.repo
  ghWorkflowId.value = c.github.workflowId
  ghRef.value = c.github.gitRef
}

watch(mirrorOptions, (opts) => {
  const c = config.value
  if (!c) return
  selMirror.value = opts.find((m) => m.Tag === c.mirror.activeId) ?? opts[0] ?? null
})

onMounted(async () => {
  await loadConfig()
  syncFromConfig()
})
</script>
