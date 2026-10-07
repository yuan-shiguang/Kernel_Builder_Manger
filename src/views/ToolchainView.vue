<template>
  <div>
    <h1 class="page-title">工具链</h1>
    <p class="page-subtitle">
      自动识别内核 Linux 版本 → 匹配 Clang 与 GCC；缺失组件一键下载（googlesource 主源 + GitHub 镜像回退）。
    </p>

    <InfoBar
      v-if="message"
      :IsOpen="true"
      :Severity="severity"
      :Title="severity === 'Error' ? '操作失败' : '完成'"
      :Message="message"
      :IsClosable="true"
      @CloseButtonClick="clear"
    />

    <!-- 当前方案 -->
    <div class="card">
      <div class="section-title">当前内核的工具链方案</div>
      <p style="margin: 0 0 10px">{{ plan?.reason || '尚未识别内核版本' }}</p>
      <table class="kv-table">
        <thead>
          <tr><th>组件</th><th>版本</th><th>状态</th><th>路径 / 操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="it in planItems" :key="it.id">
            <td>{{ it.kind === 'clang' ? 'Clang' : `GCC（${it.arch === 'arm32' ? 'arm32' : 'aarch64'}）` }}</td>
            <td class="mono">{{ it.version }}</td>
            <td>
              <span class="badge" :class="it.installed ? 'badge-ok' : 'badge-err'">
                {{ it.installed ? '已安装' : '缺失' }}
              </span>
            </td>
            <td>
              <span class="mono muted">{{ it.path }}</span>
              <Button v-if="!it.installed" Content="下载" @Click="install(it.id)" />
            </td>
          </tr>
        </tbody>
      </table>
      <div class="row" style="margin-top: 12px">
        <Button Content="一键补齐缺失组件" :IsEnabled="!busy" @Click="installMissing" />
        <Button Content="刷新" @Click="refresh" />
      </div>
    </div>

    <!-- 版本对应表 -->
    <div class="card section">
      <div class="section-title">内核版本 ↔ 工具链对应表</div>
      <table class="kv-table">
        <thead>
          <tr><th>内核版本</th><th>Clang</th><th>GCC</th></tr>
        </thead>
        <tbody>
          <tr><td class="mono">未知 / 兜底</td><td>AOSP Clang r365631c（最兼容）</td><td class="mono">4.9</td></tr>
          <tr><td class="mono">4.4</td><td>Clang 9</td><td class="mono">4.4</td></tr>
          <tr><td class="mono">4.9</td><td>Clang 9</td><td class="mono">4.9</td></tr>
          <tr><td class="mono">4.14</td><td>Clang 11</td><td class="mono">4.14</td></tr>
          <tr><td class="mono">4.19</td><td>Clang 12（r383902）</td><td class="mono">4.19</td></tr>
          <tr><td class="mono">5.4</td><td>Clang 12（r383902）</td><td class="mono">5.4</td></tr>
          <tr><td class="mono">5.10</td><td>Clang 14（r416183b）</td><td class="mono">5.10</td></tr>
          <tr><td class="mono">5.15</td><td>Clang 17（r450784e / r487747c）</td><td class="mono">5.15</td></tr>
          <tr><td class="mono">6.1</td><td>Clang 17（r450784e / r487747c）</td><td class="mono">6.1</td></tr>
        </tbody>
      </table>
    </div>

    <!-- 手动覆盖 -->
    <div class="card section">
      <div class="section-title">手动覆盖</div>
      <div class="row">
        <ComboBox
          :ItemsSource="clangOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selClang"
          Header="Clang（留空=自动）"
          Width="360"
        />
        <ComboBox
          :ItemsSource="gccOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selGcc"
          Header="GCC（留空=自动）"
          Width="200"
        />
        <TextBox
          v-model:Text="installDir"
          Header="安装目录"
          Width="320"
        />
      </div>
      <div class="row" style="margin-top: 12px">
        <Button Content="保存覆盖设置" @Click="saveOverrides" />
      </div>
    </div>

    <!-- 全部工具链 -->
    <div class="card section">
      <div class="section-title">全部组件（{{ items.length }}）</div>
      <table class="kv-table">
        <thead>
          <tr>
            <th>名称</th><th>类型</th><th>体积</th><th>状态</th><th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="it in items" :key="it.id">
            <td>{{ it.name }}<div class="muted">{{ it.note }}</div></td>
            <td>{{ it.kind === 'clang' ? 'Clang' : `GCC · ${it.arch}` }}</td>
            <td class="muted">{{ it.sizeHint }}</td>
            <td>
              <span class="badge" :class="it.installed ? 'badge-ok' : 'badge-mute'">
                {{ it.installed ? '已安装' : '未安装' }}
              </span>
            </td>
            <td>
              <Button v-if="!it.installed" Content="下载" @Click="install(it.id)" />
              <span v-else class="muted mono">{{ it.path }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 主机依赖 -->
    <div class="card section">
      <div class="section-title">主机依赖</div>
      <div class="row" style="gap: 6px">
        <span v-for="d in deps" :key="d.name" class="badge" :class="depClass(d)">{{ d.name }}</span>
      </div>
      <p v-if="missingDeps.length" class="muted" style="margin-top: 10px">
        缺少必需依赖：{{ missingDeps.join('、') }}
      </p>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">下载 / 安装日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import {
  hostDependencies,
  toolchainInstall,
  toolchainInstallMissing,
  toolchainList,
  toolchainPlan,
} from '@/api/tauri'
import { config, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const plan = ref<T.ToolchainPlan | null>(null)
const items = ref<T.ToolchainItem[]>([])
const deps = ref<T.HostDependency[]>([])
const installDir = ref('')

const planItems = computed(() => {
  if (!plan.value) return []
  return [plan.value.clang, plan.value.gccAarch64, plan.value.gccArm32].filter(
    (x): x is T.ToolchainItem => !!x,
  )
})

const missingDeps = computed(() => deps.value.filter((d) => d.required && !d.present).map((d) => d.name))
const depClass = (d: T.HostDependency) =>
  d.present ? 'badge-ok' : d.required ? 'badge-err' : 'badge-mute'

const CLANG_TAGS = [
  { Tag: '', Content: '自动（按内核版本）' },
  { Tag: 'clang-r365631c', Content: 'AOSP Clang r365631c（最兼容）' },
  { Tag: 'clang-9', Content: 'Clang 9' },
  { Tag: 'clang-11', Content: 'Clang 11' },
  { Tag: 'clang-r383902', Content: 'Clang 12（r383902）' },
  { Tag: 'clang-r416183b', Content: 'Clang 14（r416183b）' },
  { Tag: 'clang-r450784e', Content: 'Clang 17（r450784e）' },
  { Tag: 'clang-r487747c', Content: 'Clang 17（r487747c）' },
]
const clangOptions = CLANG_TAGS
const selClang = ref(CLANG_TAGS[0])

const gccOptions = [
  { Tag: '', Content: '自动（按内核版本）' },
  { Tag: '4.4', Content: 'GCC 4.4' },
  { Tag: '4.9', Content: 'GCC 4.9' },
  { Tag: '4.14', Content: 'GCC 4.14' },
  { Tag: '4.19', Content: 'GCC 4.19' },
  { Tag: '5.4', Content: 'GCC 5.4' },
  { Tag: '5.10', Content: 'GCC 5.10' },
  { Tag: '5.15', Content: 'GCC 5.15' },
  { Tag: '6.1', Content: 'GCC 6.1' },
]
const selGcc = ref(gccOptions[0])

async function refresh() {
  await run(async () => {
    plan.value = await toolchainPlan()
    items.value = await toolchainList()
    deps.value = await hostDependencies()
  })
}

async function install(id: string) {
  await run(async () => {
    await toolchainInstall(id)
    await refresh()
  }, '安装完成')
}

async function installMissing() {
  await run(async () => {
    await toolchainInstallMissing()
    await refresh()
  }, '缺失组件补齐完成')
}

async function saveOverrides() {
  if (!config.value) return
  config.value.toolchain.clangOverride = selClang.value?.Tag ?? ''
  config.value.toolchain.gccOverride = selGcc.value?.Tag ?? ''
  config.value.toolchain.installDir = installDir.value
  await persist()
  await run(async () => {
    plan.value = await toolchainPlan()
    items.value = await toolchainList()
  }, '已保存并重新计算方案')
}

onMounted(async () => {
  await refresh()
  if (config.value) {
    installDir.value = config.value.toolchain.installDir
    selClang.value =
      CLANG_TAGS.find((c) => c.Tag === config.value!.toolchain.clangOverride) ?? CLANG_TAGS[0]
    selGcc.value = gccOptions.find((g) => g.Tag === config.value!.toolchain.gccOverride) ?? gccOptions[0]
  }
})
</script>
