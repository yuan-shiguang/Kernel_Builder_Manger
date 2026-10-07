<template>
  <div>
    <h1 class="page-title">内核源码</h1>
    <p class="page-subtitle">
      默认 HTTPS 拉取；检测到本机已配置 SSH Key 时自动切换 SSH。可在设置中关闭 <code>--depth=1</code> 浅克隆。
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
      <!-- 克隆表单 -->
      <div class="card">
        <div class="section-title">拉取内核源码</div>

        <TextBox
          v-model:Text="form.repo"
          Header="仓库"
          PlaceholderText="owner/repo 或 https://github.com/owner/repo"
        />
        <TextBox v-model:Text="form.branch" Header="分支" PlaceholderText="main" />

        <ComboBox
          :ItemsSource="protocolOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selProtocol"
          Header="协议"
          Width="220"
        />
        <p class="muted" style="margin: 4px 0 10px">
          当前 SSH 状态：
          <span class="badge" :class="ssh?.hasKey ? 'badge-ok' : 'badge-mute'">
            {{ ssh?.hasKey ? `已检测到 Key（${ssh.keys.join(', ')}）` : '未检测到 SSH Key' }}
          </span>
          <span v-if="ssh?.agentLoaded" class="badge badge-ok">ssh-agent 已加载</span>
        </p>

        <ToggleSwitch v-model:IsOn="form.shallow" Header="浅克隆" OnContent="--depth=1" OffContent="完整历史" />
        <TextBox
          v-if="form.shallow"
          v-model:Text="depthText"
          Header="克隆深度"
          PlaceholderText="1"
          Width="140"
        />
        <ToggleSwitch v-model:IsOn="form.singleBranch" Header="单分支" OnContent="--single-branch" OffContent="全部分支" />
        <ToggleSwitch v-model:IsOn="form.submodules" Header="子模块" OnContent="递归拉取" OffContent="不拉取" />

        <TextBox
          v-model:Text="form.target"
          Header="目标目录"
          PlaceholderText="/home/user/KernelBuilder/kernel"
        />

        <div class="row" style="margin-top: 14px">
          <Button Content="选择目录…" @Click="pickDir" />
          <Button Content="开始克隆" :IsEnabled="!busy" @Click="doClone" />
          <Button Content="取消" :IsEnabled="busy" @Click="cancelClone" />
        </div>

        <div v-if="previewUrl" class="code-block" style="margin-top: 12px">{{ previewUrl }}</div>
      </div>

      <!-- 已有源码 -->
      <div class="card">
        <div class="section-title">使用已有源码</div>
        <TextBox v-model:Text="localDir" Header="内核源码目录" PlaceholderText="选择本地内核源码" />
        <div class="row" style="margin-top: 12px">
          <Button Content="浏览…" @Click="pickLocal" />
          <Button Content="识别" @Click="doScan" />
          <Button Content="设为当前内核" :IsEnabled="!!kernel?.valid" @Click="setAsKernel" />
        </div>

        <table v-if="kernel?.valid" class="kv-table" style="margin-top: 14px">
          <tbody>
            <tr><td>Linux 版本</td><td class="mono">{{ kernel.fullVersion }}</td></tr>
            <tr><td>架构</td><td class="mono">{{ kernel.archs.join(', ') }}</td></tr>
            <tr><td>defconfig</td><td class="mono">{{ kernel.defconfigs.length }} 个</td></tr>
          </tbody>
        </table>
        <p v-else class="muted" style="margin-top: 12px">{{ kernel?.message || '' }}</p>
      </div>
    </div>

    <!-- 下载进度 -->
    <div v-if="dl" class="card section">
      <div class="section-title">{{ dl.label }}</div>
      <ProgressBar :Value="dl.percent" :Maximum="100" />
      <p class="muted" style="margin-top: 6px">
        {{ formatSize(dl.received) }} / {{ formatSize(dl.total) }}（{{ dl.percent.toFixed(1) }}%）
      </p>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">克隆日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import {
  cancelTask,
  cloneKernel,
  detectSsh,
  formatSize,
  previewCloneUrl,
  scanKernel,
  setKernelDir,
} from '@/api/tauri'
import { config, persist, workspaceDir } from '@/stores/settings'
import { downloads } from '@/stores/log'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const ssh = ref<T.SshStatus | null>(null)
const kernel = ref<T.KernelInfo | null>(null)
const previewUrl = ref('')
const localDir = ref('')
const depthText = ref('1')

const form = ref({
  repo: '',
  branch: 'main',
  protocol: 'auto',
  shallow: true,
  singleBranch: true,
  submodules: false,
  target: '',
})

const protocolOptions = [
  { Tag: 'auto', Content: '自动（有 SSH Key 用 SSH）' },
  { Tag: 'https', Content: '强制 HTTPS' },
  { Tag: 'ssh', Content: '强制 SSH' },
]
const selProtocol = ref(protocolOptions[0])
watch(selProtocol, (v) => {
  form.value.protocol = v?.Tag ?? 'auto'
})

const dl = computed(() => Object.values(downloads.value).find((d) => !d.done) ?? null)

async function refreshPreview() {
  if (!form.value.repo) {
    previewUrl.value = ''
    return
  }
  try {
    previewUrl.value = await previewCloneUrl(form.value.repo, form.value.protocol)
  } catch {
    previewUrl.value = ''
  }
}
watch(() => [form.value.repo, form.value.protocol], refreshPreview)

async function pickDir() {
  const picked = await openDialog({ directory: true, multiple: false, title: '选择克隆目标目录' })
  if (typeof picked === 'string') form.value.target = picked
}
async function pickLocal() {
  const picked = await openDialog({ directory: true, multiple: false, title: '选择内核源码目录' })
  if (typeof picked === 'string') {
    localDir.value = picked
    await doScan()
  }
}

async function doClone() {
  if (!form.value.repo) return fail('请填写仓库地址')
  if (!form.value.target) return fail('请选择目标目录')
  await run(async () => {
    await cloneKernel({
      repo: form.value.repo,
      branch: form.value.branch,
      protocol: form.value.protocol,
      shallow: form.value.shallow,
      depth: Number(depthText.value) || 1,
      singleBranch: form.value.singleBranch,
      submodules: form.value.submodules,
      target: form.value.target,
    })
    // 保存源码配置
    if (config.value) {
      config.value.source.repo = form.value.repo
      config.value.source.branch = form.value.branch
      config.value.source.protocol = form.value.protocol
      config.value.source.shallow = form.value.shallow
      config.value.source.depth = Number(depthText.value) || 1
      config.value.source.singleBranch = form.value.singleBranch
      config.value.source.submodules = form.value.submodules
      await persist()
    }
    kernel.value = await scanKernel(form.value.target)
    localDir.value = form.value.target
  }, '克隆完成')
}

async function cancelClone() {
  await run(() => cancelTask('clone'))
}

async function doScan() {
  if (!localDir.value) return fail('请先选择目录')
  await run(async () => {
    kernel.value = await scanKernel(localDir.value)
  })
}

async function setAsKernel() {
  if (!localDir.value) return
  await run(async () => {
    kernel.value = await setKernelDir(localDir.value)
    ok('已设为当前内核目录')
  })
}

onMounted(async () => {
  ssh.value = await detectSsh()
  if (config.value) {
    form.value.repo = config.value.source.repo
    form.value.branch = config.value.source.branch
    form.value.protocol = config.value.source.protocol
    form.value.shallow = config.value.source.shallow
    depthText.value = String(config.value.source.depth || 1)
    form.value.singleBranch = config.value.source.singleBranch
    form.value.submodules = config.value.source.submodules
    localDir.value = config.value.general.kernelDir
    if (!form.value.target && workspaceDir.value) {
      const name = config.value.source.repo.split('/').pop() || 'kernel'
      form.value.target = `${workspaceDir.value}/${name}`
    }
    selProtocol.value =
      protocolOptions.find((p) => p.Tag === config.value!.source.protocol) ?? protocolOptions[0]
  }
  await refreshPreview()
})
</script>
