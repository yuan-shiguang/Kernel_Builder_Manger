<template>
  <div>
    <h1 class="page-title">概览</h1>
    <p class="page-subtitle">内核构建工作区总览：源码、工具链、补丁与构建状态一目了然。</p>

    <InfoBar
      v-if="message"
      :IsOpen="true"
      :Severity="severity"
      :Title="severity === 'Error' ? '操作失败' : '操作完成'"
      :Message="message"
      :IsClosable="true"
      @CloseButtonClick="clear"
    />

    <!-- 状态卡片 -->
    <div class="row" style="margin-bottom: 18px">
      <StatCard
        label="内核版本"
        :value="kernel?.fullVersion || '未识别'"
        :hint="kernel?.message || ''"
        :tone="kernel?.valid ? 'ok' : 'mute'"
      />
      <StatCard
        label="内核目录"
        :value="shortDir(kernelDir)"
        :hint="kernelDir || '尚未选择'"
        :tone="kernelDir ? 'ok' : 'warn'"
      />
      <StatCard
        label="工具链"
        :value="plan?.clang?.installed ? '已就绪' : '缺失'"
        :hint="plan?.reason || ''"
        :tone="plan && plan.missing.length === 0 ? 'ok' : 'warn'"
      />
      <StatCard
        label="KernelSU"
        :value="ksu?.integrated ? '已集成' : '未集成'"
        :hint="ksu?.message || ''"
        :tone="ksu?.integrated ? 'ok' : 'mute'"
      />
      <StatCard
        label="缺失依赖"
        :value="String(missingCount)"
        :hint="missingCount ? '见下方依赖检查' : '全部就位'"
        :tone="missingCount ? 'warn' : 'ok'"
      />
    </div>

    <!-- 缺失工具链警告 -->
    <InfoBar
      v-if="plan && plan.missing.length"
      :IsOpen="true"
      Severity="Warning"
      Title="工具链缺失"
      :Message="`检测到 ${plan.missing.length} 个组件未安装：${plan.missing.map((m) => m.name).join('、')}`"
      :IsClosable="false"
    >
      <div style="margin-top: 8px">
        <Button Content="一键下载缺失组件" @Click="installMissing" />
      </div>
    </InfoBar>

    <div class="grid-2">
      <!-- 快速操作 -->
      <div class="card">
        <div class="section-title">快速操作</div>
        <div class="row">
          <Button Content="扫描内核目录" @Click="rescan" />
          <Button Content="重新初始化" @Click="bootstrap" />
          <Button Content="打开工作区" @Click="openWorkspace" />
          <Button Content="打开内核目录" @Click="openKernel" />
        </div>
        <p class="muted" style="margin-top: 10px">
          工作区：{{ workspaceDir }}
        </p>
      </div>

      <!-- 内核信息 -->
      <div class="card">
        <div class="section-title">内核信息</div>
        <table v-if="kernel?.valid" class="kv-table">
          <tbody>
            <tr><td>Linux 版本</td><td class="mono">{{ kernel.fullVersion }}</td></tr>
            <tr><td>可用架构</td><td class="mono">{{ kernel.archs.join(', ') }}</td></tr>
            <tr><td>defconfig 数量</td><td class="mono">{{ kernel.defconfigs.length }}</td></tr>
            <tr><td>当前 defconfig</td><td class="mono">{{ config?.build.defconfig || '未选择' }}</td></tr>
          </tbody>
        </table>
        <p v-else class="muted">{{ kernel?.message || '点击「扫描内核目录」识别源码' }}</p>
      </div>
    </div>

    <!-- 工作流进度 -->
    <div class="card section">
      <div class="section-title">工作流进度</div>
      <div class="flow">
        <div
          v-for="(s, i) in flow"
          :key="s.title"
          class="flow-step"
          :class="`flow-${s.state}`"
          @click="go(s.route)"
        >
          <div class="flow-idx">{{ i + 1 }}</div>
          <div class="flow-title">{{ s.title }}</div>
          <div class="muted">{{ s.hint }}</div>
        </div>
      </div>
    </div>

    <!-- 主机依赖 -->
    <div class="card section">
      <div class="section-title">主机依赖检查</div>
      <div class="row" style="gap: 6px">
        <span v-for="d in deps" :key="d.name" class="badge" :class="badgeClass(d)">
          {{ d.name }}
        </span>
      </div>
      <p v-if="missingDeps.length" class="muted" style="margin-top: 10px">
        缺少必需依赖：{{ missingDeps.join('、') }}
      </p>
    </div>

    <!-- 实时日志 -->
    <div class="section" style="height: 320px">
      <div class="section-title">实时日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import LogPanel from '@/components/LogPanel.vue'
import StatCard from '@/components/StatCard.vue'
import { useTask } from '@/composables/useTask'
import {
  bootstrapNow,
  hostDependencies,
  openPath,
  scanKernel,
  toolchainInstallMissing,
  toolchainPlan,
  ksuStatus,
} from '@/api/tauri'
import { config, kernelDir, persist, workspaceDir } from '@/stores/settings'
import type * as T from '@/types'

const router = useRouter()
const { busy, message, severity, run, clear, ok, fail } = useTask()

const kernel = ref<T.KernelInfo | null>(null)
const plan = ref<T.ToolchainPlan | null>(null)
const ksu = ref<T.KsuStatus | null>(null)
const deps = ref<T.HostDependency[]>([])

type StepState = 'done' | 'todo' | 'warn'

/** 端到端工作流：源码 → 工具链 → 补丁 → KSU → 构建 */
const flow = computed<
  { title: string; hint: string; state: StepState; route: string }[]
>(() => [
  {
    title: '源码',
    hint: kernelDir.value ? (kernel.value?.fullVersion ?? '已选择') : '未选择内核目录',
    state: kernelDir.value ? 'done' : 'todo',
    route: 'source',
  },
  {
    title: '工具链',
    hint: plan.value
      ? plan.value.missing.length
        ? `缺 ${plan.value.missing.length} 项`
        : '已就绪'
      : '未检测',
    state: plan.value ? (plan.value.missing.length ? 'warn' : 'done') : 'todo',
    route: 'toolchain',
  },
  {
    title: 'SUSFS',
    hint: config.value?.susfs.repo || '未配置',
    state: 'todo',
    route: 'susfs',
  },
  {
    title: 'KernelSU',
    hint: ksu.value?.integrated ? `${ksu.value.provider} 已集成` : '未集成',
    state: ksu.value?.integrated ? 'done' : 'todo',
    route: 'ksu',
  },
  {
    title: 'defconfig',
    hint: config.value?.build.defconfig || '未选择',
    state: config.value?.build.defconfig ? 'done' : 'todo',
    route: 'defconfig',
  },
  {
    title: '构建',
    hint: `${config.value?.build.arch ?? 'arm64'} → ${config.value?.build.target ?? 'Image'}`,
    state: 'todo',
    route: 'build',
  },
])

function go(route: string) {
  void router.push({ name: route })
}

const missingDeps = computed(() => deps.value.filter((d) => d.required && !d.present).map((d) => d.name))
const missingCount = computed(() => missingDeps.value.length + (plan.value?.missing.length ?? 0))

function shortDir(p: string) {
  if (!p) return '—'
  return p.length > 28 ? '…' + p.slice(-27) : p
}

function badgeClass(d: T.HostDependency) {
  if (d.present) return 'badge-ok'
  return d.required ? 'badge-err' : 'badge-mute'
}

async function refresh() {
  await run(async () => {
    deps.value = await hostDependencies()
    plan.value = await toolchainPlan()
    ksu.value = await ksuStatus()
    if (kernelDir.value) kernel.value = await scanKernel(kernelDir.value)
  })
}

async function rescan() {
  if (!kernelDir.value) {
    fail('未选择内核目录，请先到「内核源码」页拉取或选择源码')
    return
  }
  await run(async () => {
    kernel.value = await scanKernel(kernelDir.value)
    plan.value = await toolchainPlan()
  })
}

async function installMissing() {
  await run(async () => {
    await toolchainInstallMissing()
    plan.value = await toolchainPlan()
  }, '工具链安装完成')
}

async function bootstrap() {
  await run(async () => {
    const r = await bootstrapNow(true)
    if (!r.ok) {
      fail(`初始化未完全成功：${r.failed.join('；') || '缺少主机依赖'}`)
    } else {
      ok('初始化完成')
    }
    await refresh()
    await persist()
  })
}

async function openWorkspace() {
  await run(() => openPath(workspaceDir.value))
}
async function openKernel() {
  if (!kernelDir.value) return fail('未选择内核目录')
  await run(() => openPath(kernelDir.value))
}

onMounted(refresh)
</script>

<style scoped>
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
}

.flow-step {
  border: 1px solid var(--card-stroke, #3a3a3a);
  border-left-width: 3px;
  border-radius: 6px;
  padding: 10px 12px;
  cursor: pointer;
  transition: background 0.12s ease;
}

.flow-step:hover {
  background: var(--subtle-secondary);
}

.flow-idx {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  background: var(--subtle-secondary);
  margin-bottom: 6px;
}

.flow-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 2px;
}

.flow-done {
  border-left-color: var(--SystemFillColorSuccessBrush);
}

.flow-warn {
  border-left-color: var(--SystemFillColorCautionBrush);
}

.flow-todo {
  border-left-color: var(--stroke-divider, #454545);
}
</style>
