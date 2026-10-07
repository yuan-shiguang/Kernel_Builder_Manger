<template>
  <div>
    <h1 class="page-title">构建内核</h1>
    <p class="page-subtitle">
      依据内核 Linux 版本自动匹配 Clang / GCC 工具链并生成构建脚本；编译过程实时回显到日志，错误自动高亮。
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

    <InfoBar
      v-if="!kernelDir"
      :IsOpen="true"
      Severity="Warning"
      Title="尚未选择内核"
      Message="请先到「内核源码」页拉取或选择内核源码目录，再回到本页构建。"
      :IsClosable="false"
    />

    <InfoBar
      v-if="plan && plan.missing.length"
      :IsOpen="true"
      Severity="Warning"
      Title="工具链缺失"
      :Message="`以下组件未安装：${plan.missing.map((m) => m.name).join('、')}。请先到「工具链」页下载。`"
      :IsClosable="false"
    />

    <!-- 状态条 -->
    <div class="row section">
      <StatCard
        label="内核版本"
        :value="kernel?.fullVersion || '未识别'"
        :hint="kernel?.message || ''"
        :tone="kernel?.valid ? 'ok' : 'mute'"
      />
      <StatCard
        label="Clang"
        :value="plan?.clang?.version || '未匹配'"
        :hint="plan?.clang?.name || plan?.reason || ''"
        :tone="plan?.clang?.installed ? 'ok' : 'warn'"
      />
      <StatCard
        label="架构 / 目标"
        :value="`${cfg.arch} → ${cfg.target || '—'}`"
        :hint="cfg.defconfig || '未选择 defconfig'"
        tone="mute"
      />
      <StatCard
        label="构建状态"
        :value="building ? '进行中' : '空闲'"
        :hint="building ? '详见下方日志' : '等待开始'"
        :tone="building ? 'warn' : 'mute'"
      />
    </div>

    <div class="grid-2">
      <!-- 构建参数 -->
      <div class="card">
        <div class="section-title">构建参数</div>

        <template v-if="kernelDir">
          <div class="row">
            <ComboBox
              :ItemsSource="archOptions"
              DisplayMemberPath="Content"
              v-model:SelectedItem="selArch"
              Header="架构 ARCH"
              Width="180"
            />
            <TextBox v-model:Text="cfg.defconfig" Header="defconfig" Width="240" />
          </div>

          <div class="row">
            <TextBox v-model:Text="cfg.target" Header="目标产物" Width="170" />
            <TextBox v-model:Text="jobsText" Header="并行任务（0=自动）" Width="170" />
            <TextBox v-model:Text="cfg.outDir" Header="输出目录" Width="140" />
          </div>

          <div class="row">
            <ToggleSwitch v-model:IsOn="cfg.useCcache" Header="ccache" OnContent="启用" OffContent="关闭" />
            <ToggleSwitch v-model:IsOn="cfg.useLto" Header="LTO" OnContent="启用" OffContent="关闭" />
            <ToggleSwitch
              v-model:IsOn="cfg.packageAnykernel"
              Header="打包 AnyKernel3"
              OnContent="是"
              OffContent="否"
            />
          </div>

          <TextBox
            v-model:Text="cfg.extraArgs"
            Header="附加编译参数"
            PlaceholderText="例如 LLVM=1 LLVM_IAS=1"
          />
          <TextBox
            v-if="cfg.packageAnykernel"
            v-model:Text="cfg.anykernelRepo"
            Header="AnyKernel3 仓库"
            Width="320"
          />

          <div class="row" style="margin-top: 12px">
            <Button Content="保存配置" :IsEnabled="!busy" @Click="saveCfg" />
            <Button Content="预览构建脚本" :IsEnabled="!busy" @Click="preview" />
            <Button Content="开始构建" :IsEnabled="!busy" @Click="start" />
            <Button Content="取消" :IsEnabled="building" @Click="cancel" />
          </div>
          <div class="row" style="margin-top: 8px">
            <Button Content="清理产物" :IsEnabled="!building" @Click="clean" />
            <Button Content="打开输出目录" :IsEnabled="!!outputPath" @Click="openOut" />
          </div>
        </template>
        <p v-else class="muted">尚未选择内核目录。</p>
      </div>

      <!-- 脚本预览 -->
      <div class="card">
        <div class="section-title">
          构建脚本预览
          <span class="spacer" />
          <Button v-if="scriptPreview" Content="复制" @Click="copyScript" />
        </div>
        <pre v-if="scriptPreview" class="code-block" style="max-height: 320px; overflow: auto">{{
          scriptPreview
        }}</pre>
        <p v-else class="muted">点击「预览构建脚本」查看将要执行的编译命令。</p>

        <div class="section-title" style="margin-top: 16px">工具链匹配</div>
        <table class="kv-table">
          <tbody>
            <tr>
              <td style="width: 130px">Clang</td>
              <td class="mono">
                {{ plan?.clang ? `${plan.clang.name}（${plan.clang.installed ? '已安装' : '缺失'}）` : '—' }}
              </td>
            </tr>
            <tr>
              <td>GCC aarch64</td>
              <td class="mono">
                {{
                  plan?.gccAarch64
                    ? `${plan.gccAarch64.name}（${plan.gccAarch64.installed ? '已装' : '缺失'}）`
                    : '—'
                }}
              </td>
            </tr>
            <tr>
              <td>匹配依据</td>
              <td class="muted">{{ plan?.reason || '—' }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 产物 -->
    <div class="card section">
      <div class="row-between">
        <div class="section-title" style="margin: 0">构建产物（{{ artifacts.length }}）</div>
        <div class="row">
          <Button Content="刷新" @Click="loadArtifacts" />
          <Button
            Content="打包为 AnyKernel3"
            :IsEnabled="!!artifacts.length && !busy"
            @Click="doPackage"
          />
        </div>
      </div>
      <p v-if="outputPath" class="muted mono" style="margin: 6px 0 10px">{{ outputPath }}</p>

      <table v-if="artifacts.length" class="kv-table">
        <thead>
          <tr>
            <th>产物</th>
            <th style="width: 110px">大小</th>
            <th style="width: 90px">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="a in artifacts" :key="a.path">
            <td class="mono" style="word-break: break-all">{{ a.name }}</td>
            <td class="mono">{{ formatSize(a.size) }}</td>
            <td><Button Content="打开" @Click="openFile(a.path)" /></td>
          </tr>
        </tbody>
      </table>
      <p v-else class="muted">暂无产物。构建成功后点击「刷新」。</p>

      <div v-if="packaged" class="row" style="margin-top: 10px">
        <span class="badge badge-ok">刷机包已生成</span>
        <span class="mono muted">{{ packaged }}</span>
      </div>
    </div>

    <div class="section" style="height: 340px">
      <div class="section-title">
        构建日志
        <span v-if="building" class="badge badge-warn">运行中</span>
      </div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import StatCard from '@/components/StatCard.vue'
import { useTask } from '@/composables/useTask'
import {
  buildArtifacts,
  buildClean,
  buildOutputPath,
  buildPackage,
  buildPreview,
  buildStart,
  cancelTask,
  formatSize,
  openPath,
  scanKernel,
  toolchainPlan,
} from '@/api/tauri'
import { config, kernelDir, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const building = ref(false)
const scriptPreview = ref('')
const outputPath = ref('')
const packaged = ref('')
const jobsText = ref('0')
const artifacts = ref<T.ArtifactInfo[]>([])
const kernel = ref<T.KernelInfo | null>(null)
const plan = ref<T.ToolchainPlan | null>(null)

const cfg = reactive({
  arch: 'arm64',
  defconfig: '',
  target: 'Image',
  jobs: 0,
  useCcache: true,
  useLto: false,
  outDir: 'out',
  extraArgs: '',
  packageAnykernel: false,
  anykernelRepo: 'osm0sis/AnyKernel3',
})

const archOptions = [
  { Tag: 'arm', Content: 'arm（32 位）' },
  { Tag: 'arm64', Content: 'arm64' },
  { Tag: 'x86_64', Content: 'x86_64' },
  { Tag: 'x86', Content: 'x86' },
  { Tag: 'riscv', Content: 'riscv' },
]
const selArch = ref(archOptions[1])

watch(selArch, (v) => {
  cfg.arch = v?.Tag ?? 'arm64'
})

watch(jobsText, (v) => {
  cfg.jobs = Math.max(0, parseInt(v || '0', 10) || 0)
})

async function syncConfig() {
  if (!config.value) return
  const b = config.value.build
  b.arch = cfg.arch
  b.defconfig = cfg.defconfig
  b.target = cfg.target
  b.jobs = cfg.jobs
  b.useCcache = cfg.useCcache
  b.useLto = cfg.useLto
  b.packageAnykernel = cfg.packageAnykernel
  b.anykernelRepo = cfg.anykernelRepo
  b.outDir = cfg.outDir
  b.extraArgs = cfg.extraArgs
  await persist()
}

async function saveCfg() {
  await syncConfig()
  ok('构建配置已保存')
}

async function preview() {
  await syncConfig()
  await run(async () => {
    scriptPreview.value = await buildPreview()
  })
}

async function start() {
  if (!kernelDir.value) return fail('尚未选择内核目录')
  await syncConfig()
  await run(async () => {
    const code = await buildStart()
    if (code === 0) ok('构建完成')
    else fail(`构建失败（退出码 ${code}），详见日志中高亮的错误行`)
  })
  building.value = false
  await loadArtifacts()
}

async function cancel() {
  await run(async () => {
    const done = await cancelTask('build')
    building.value = false
    if (done) ok('已发送取消信号（先 TERM 后 KILL）')
    else fail('取消失败：构建进程可能已结束')
  })
}

async function clean() {
  await run(async () => {
    await buildClean()
    packaged.value = ''
    await loadArtifacts()
  }, '已清理构建产物')
}

async function doPackage() {
  await run(async () => {
    packaged.value = await buildPackage()
  }, 'AnyKernel3 刷机包已生成')
}

async function loadArtifacts() {
  try {
    outputPath.value = await buildOutputPath()
  } catch {
    outputPath.value = ''
  }
  try {
    artifacts.value = await buildArtifacts()
  } catch {
    artifacts.value = []
  }
}

async function openOut() {
  if (!outputPath.value) await loadArtifacts()
  if (outputPath.value) await openPath(outputPath.value)
}

async function openFile(p: string) {
  await run(() => openPath(p))
}

async function copyScript() {
  await navigator.clipboard?.writeText(scriptPreview.value)
  ok('脚本已复制到剪贴板')
}

async function refreshPlan() {
  try {
    plan.value = await toolchainPlan()
    if (kernelDir.value) kernel.value = await scanKernel(kernelDir.value)
  } catch {
    /* 预览模式忽略 */
  }
}

onMounted(async () => {
  if (config.value) {
    const b = config.value.build
    Object.assign(cfg, {
      arch: b.arch,
      defconfig: b.defconfig,
      target: b.target,
      jobs: b.jobs,
      useCcache: b.useCcache,
      useLto: b.useLto,
      outDir: b.outDir,
      extraArgs: b.extraArgs,
      packageAnykernel: b.packageAnykernel,
      anykernelRepo: b.anykernelRepo,
    })
    jobsText.value = String(b.jobs)
    selArch.value = archOptions.find((a) => a.Tag === b.arch) ?? archOptions[1]
  }
  await refreshPlan()
  await loadArtifacts()
})
</script>
