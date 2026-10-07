<template>
  <div>
    <h1 class="page-title">KernelSU</h1>
    <p class="page-subtitle">
      内置 8 个主流分支源，分支与标签通过 GitHub API 动态拉取 —— 上游新增分支无需更新本程序。
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

    <div class="grid-2">
      <!-- 集成 -->
      <div class="card">
        <div class="section-title">选择分支源</div>

        <ComboBox
          :ItemsSource="providerOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selProvider"
          Header="Provider"
          Width="320"
        />
        <p class="muted" style="margin: 6px 0 10px">
          {{ currentProvider?.description }}
          <br />
          <span class="mono">{{ currentProvider?.owner }}/{{ currentProvider?.repo }}</span>
          · {{ currentProvider?.author }}
        </p>

        <div class="row">
          <TextBox v-model:Text="ownerOverride" Header="Owner（可覆盖）" Width="160" />
          <TextBox v-model:Text="repoOverride" Header="Repo（可覆盖）" Width="200" />
        </div>

        <ComboBox
          :ItemsSource="refOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selRef"
          Header="分支 / 标签"
          Width="320"
        />
        <div class="row" style="margin-top: 8px">
          <Button Content="刷新分支列表" @Click="loadRefs" />
          <ToggleSwitch v-model:IsOn="includeTags" Header="包含标签" OnContent="是" OffContent="否" />
        </div>

        <ComboBox
          :ItemsSource="methodOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selMethod"
          Header="集成方式"
          Width="260"
        />
        <p class="muted" style="margin: 4px 0 10px">
          setup：执行仓库内的 <span class="mono">kernel/setup.sh</span>（推荐，官方与各分支通用）<br />
          manual：复制 <span class="mono">kernel/</span> 目录并逐个应用补丁
        </p>

        <div class="row">
          <Button Content="下载源码" :IsEnabled="!busy" @Click="doFetch" />
          <Button Content="集成到内核" :IsEnabled="!busy" @Click="doIntegrate" />
        </div>
        <p v-if="ksuSrc" class="muted mono" style="margin-top: 8px">源码目录：{{ ksuSrc }}</p>
      </div>

      <!-- 状态 -->
      <div class="card">
        <div class="section-title">当前集成状态</div>
        <div class="row">
          <span class="badge" :class="status?.integrated ? 'badge-ok' : 'badge-mute'">
            {{ status?.integrated ? '已集成' : '未集成' }}
          </span>
          <span v-if="status?.provider" class="badge badge-mute">{{ status.provider }}</span>
        </div>
        <p class="muted" style="margin-top: 8px">{{ status?.message }}</p>

        <div v-if="status?.files.length" style="margin-top: 10px">
          <div class="muted">相关文件：</div>
          <div v-for="f in status.files" :key="f" class="mono">· {{ f }}</div>
        </div>
        <div v-if="status?.configFlags.length" style="margin-top: 10px">
          <div class="muted">配置开关：</div>
          <div v-for="c in status.configFlags" :key="c" class="mono">· {{ c }}</div>
        </div>

        <div class="row" style="margin-top: 12px">
          <Button Content="重新检测" @Click="refreshStatus" />
        </div>
      </div>
    </div>

    <!-- 管理器 APK -->
    <div class="card section">
      <div class="section-title">管理器 APK</div>
      <div class="row">
        <TextBox v-model:Text="managerRepo" Header="Release 仓库" Width="280" />
        <Button Content="列出 APK" @Click="loadAssets" />
      </div>
      <table v-if="assets.length" class="kv-table" style="margin-top: 12px">
        <thead>
          <tr><th>名称</th><th>版本</th><th>大小</th><th>操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="a in assets" :key="a.id">
            <td class="mono">{{ a.name }}</td>
            <td>{{ a.tag }}</td>
            <td>{{ formatSize(a.size) }}</td>
            <td><Button Content="下载" @Click="downloadAsset(a)" /></td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted" style="margin-top: 10px">
        点击「列出 APK」获取该分支源发布的管理器安装包（走镜像加速）。
      </p>
      <p v-if="managerPath" class="muted mono" style="margin-top: 8px">已保存：{{ managerPath }}</p>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">集成日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import {
  formatSize,
  ksuBranches,
  ksuDownloadManager,
  ksuFetch,
  ksuIntegrate,
  ksuManagerAssets,
  ksuProviders,
  ksuStatus,
} from '@/api/tauri'
import { config, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const providers = ref<T.KsuProvider[]>([])
const providerOptions = ref<{ Tag: string; Content: string }[]>([])
const selProvider = ref<{ Tag: string; Content: string } | null>(null)

const refs = ref<T.GitRefInfo[]>([])
const selRef = ref<{ Tag: string; Content: string } | null>(null)
const includeTags = ref(true)

const ownerOverride = ref('')
const repoOverride = ref('')

const methodOptions = [
  { Tag: 'setup', Content: 'setup.sh（推荐）' },
  { Tag: 'manual', Content: '手动复制 + 打补丁' },
]
const selMethod = ref(methodOptions[0])

const status = ref<T.KsuStatus | null>(null)
const ksuSrc = ref('')

const managerRepo = ref('')
const assets = ref<T.ReleaseAsset[]>([])
const managerPath = ref('')

const currentProvider = computed(
  () => providers.value.find((p) => p.id === selProvider.value?.Tag) ?? null,
)

const refOptions = computed(() =>
  refs.value.map((r) => ({
    Tag: r.name,
    Content: `${r.kind === 'tag' ? '🏷 ' : '⑂ '}${r.name}`,
  })),
)

watch(selProvider, async (v) => {
  const p = providers.value.find((x) => x.id === v?.Tag)
  if (!p) return
  ownerOverride.value = ''
  repoOverride.value = ''
  managerRepo.value = p.managerRepo
  refs.value = []
  selRef.value = null
  await loadRefs()
  const def = refs.value.find((r) => r.name === p.defaultBranch)
  selRef.value = def ? { Tag: def.name, Content: def.name } : (refOptions.value[0] ?? null)
})

async function loadRefs() {
  const pid = selProvider.value?.Tag
  if (!pid) return
  await run(async () => {
    refs.value = await ksuBranches(pid, repoOverride.value, includeTags.value)
    if (refs.value.length) {
      selRef.value = refOptions.value[0]
    }
  })
}

async function doFetch() {
  const pid = selProvider.value?.Tag
  const gitRef = selRef.value?.Tag
  if (!pid || !gitRef) return fail('请选择 Provider 与分支')
  await run(async () => {
    ksuSrc.value = await ksuFetch(pid, gitRef, ownerOverride.value, repoOverride.value)
    if (config.value) {
      config.value.ksu.provider = pid
      config.value.ksu.branch = gitRef
      config.value.ksu.method = selMethod.value.Tag
      await persist()
    }
  }, '源码下载完成')
}

async function doIntegrate() {
  if (!config.value?.general.kernelDir) return fail('请先选择内核目录')
  const pid = currentProvider.value?.id
  const gitRef = selRef.value?.Tag ?? 'main'
  if (!pid) return fail('请选择 Provider')
  await run(async () => {
    await ksuIntegrate(pid, gitRef, selMethod.value.Tag, ownerOverride.value, repoOverride.value)
    if (config.value) {
      config.value.ksu.provider = pid
      config.value.ksu.branch = gitRef
      config.value.ksu.method = selMethod.value.Tag
      await persist()
    }
    await refreshStatus()
  }, 'KernelSU 集成完成')
}

async function refreshStatus() {
  await run(async () => {
    status.value = await ksuStatus()
  })
}

async function loadAssets() {
  if (!managerRepo.value) return fail('请填写 Release 仓库')
  await run(async () => {
    assets.value = await ksuManagerAssets(managerRepo.value)
  })
}

async function downloadAsset(a: T.ReleaseAsset) {
  await run(async () => {
    managerPath.value = await ksuDownloadManager(a)
  }, `管理器已下载：${a.name}`)
}

onMounted(async () => {
  await run(async () => {
    providers.value = await ksuProviders()
    providerOptions.value = providers.value.map((p) => ({
      Tag: p.id,
      Content: `${p.name} · ${p.author}`,
    }))
    const saved = config.value?.ksu.provider ?? 'kernelsu'
    selProvider.value = providerOptions.value.find((o) => o.Tag === saved) ?? providerOptions.value[0]
    selMethod.value =
      methodOptions.find((m) => m.Tag === (config.value?.ksu.method ?? 'setup')) ?? methodOptions[0]
    managerPath.value = config.value?.ksu.managerPath ?? ''
    status.value = await ksuStatus()
  })
})
</script>
