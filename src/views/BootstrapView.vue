<template>
  <div>
    <h1 class="page-title">初始化</h1>
    <p class="page-subtitle">
      首次使用时下载必备依赖、创建目录结构并检查主机环境。全部步骤可重复执行。
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

    <!-- 步骤条 -->
    <div class="steps card section">
      <div v-for="(s, i) in steps" :key="s.key" class="step" :class="`step-${s.state}`">
        <div class="step-idx">{{ i + 1 }}</div>
        <div class="step-body">
          <div class="step-title">{{ s.title }}</div>
          <div class="muted">{{ s.desc }}</div>
        </div>
        <span class="step-mark">{{ markOf(s.state) }}</span>
      </div>
    </div>

    <div class="row section">
      <Button Content="开始 / 重新初始化" :IsEnabled="!busy" @Click="doBootstrap" />
      <Button Content="仅检查主机依赖" :IsEnabled="!busy" @Click="checkOnly" />
      <Button Content="跳过向导，进入源码页" @Click="goSource" />
      <span v-if="busy" class="muted">正在执行…</span>
    </div>

    <!-- 目录结构 -->
    <div class="grid-2 section">
      <div class="card">
        <div class="section-title">目录结构</div>
        <table class="kv-table">
          <thead>
            <tr><th>目录</th><th style="width: 100px">状态</th></tr>
          </thead>
          <tbody>
            <tr v-for="d in createdDirs" :key="d">
              <td class="mono" style="word-break: break-all">{{ d }}</td>
              <td><span class="badge badge-ok">已创建</span></td>
            </tr>
          </tbody>
        </table>
        <p v-if="!createdDirs.length" class="muted">
          工作区：<span class="mono">{{ general.workspaceDir }}</span>
        </p>
      </div>

      <div class="card">
        <div class="section-title">主机依赖检查</div>
        <table class="kv-table">
          <thead>
            <tr>
              <th>依赖</th>
              <th style="width: 90px">状态</th>
              <th>说明</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="d in deps" :key="d.name">
              <td class="mono">{{ d.name }}</td>
              <td>
                <span class="badge" :class="d.present ? 'badge-ok' : d.required ? 'badge-err' : 'badge-warn'">
                  {{ d.present ? '已安装' : d.required ? '缺失' : '可选' }}
                </span>
              </td>
              <td class="muted">{{ d.note }}</td>
            </tr>
          </tbody>
        </table>
        <p v-if="!deps.length" class="muted">点击「仅检查主机依赖」获取结果。</p>
      </div>
    </div>

    <!-- 安装提示 -->
    <div v-if="installHint" class="card section">
      <div class="section-title">安装缺失依赖</div>
      <p class="muted">在终端执行以下命令（按你的发行版选择）：</p>
      <pre class="code-block">{{ installHint }}</pre>
      <Button Content="复制命令" @Click="copyHint" />
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">初始化日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import { bootstrapNow, hostDependencies } from '@/api/tauri'
import { config, loadConfig, workspaceDir } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()
const router = useRouter()

const deps = ref<T.HostDependency[]>([])
const createdDirs = ref<string[]>([])
const installHint = ref('')

const general = computed(
  () =>
    config.value?.general ?? {
      workspaceDir: '',
      kernelDir: '',
      initialized: false,
      bootstrapped: [],
    },
)

const missingRequired = computed(() => deps.value.filter((d) => d.required && !d.present && !d.present))

type StepState = 'done' | 'doing' | 'todo' | 'fail'
const steps = computed<{ key: string; title: string; desc: string; state: StepState }[]>(() => [
  {
    key: 'deps',
    title: '主机依赖检查',
    desc: '检测 git / patch / make / zip 等必备工具',
    state: !deps.value.length ? 'todo' : missingRequired.value.length ? 'fail' : 'done',
  },
  {
    key: 'dirs',
    title: '创建工作区目录',
    desc: `在 ${general.value.workspaceDir || '工作区'} 下创建 kernel / toolchains / susfs / manager / artifacts`,
    state: createdDirs.value.length ? 'done' : 'todo',
  },
  {
    key: 'boot',
    title: '下载首启依赖',
    desc: '下载打包所需的小体积辅助文件',
    state: general.value.bootstrapped.length ? 'done' : 'todo',
  },
  {
    key: 'kernel',
    title: '拉取内核源码',
    desc: '到「内核源码」页克隆或选择本地源码',
    state: general.value.kernelDir ? 'done' : 'todo',
  },
])

function markOf(s: StepState) {
  return { done: '✓', doing: '…', todo: '○', fail: '!' }[s]
}

async function refreshDeps() {
  deps.value = await hostDependencies()
  const miss = deps.value.filter((d) => d.required && !d.present).map((d) => d.name)
  installHint.value = miss.length
    ? `# Debian / Ubuntu\nsudo apt install -y ${miss.join(' ')}\n\n# Arch\nsudo pacman -S --needed ${miss.join(' ')}\n\n# Fedora\nsudo dnf install -y ${miss.join(' ')}`
    : ''
}

async function checkOnly() {
  await run(async () => {
    await refreshDeps()
    const miss = missingRequired.value.length
    if (miss) fail(`缺少 ${miss} 个必需依赖`)
    else ok('主机依赖检查通过')
  })
}

async function doBootstrap() {
  await run(async () => {
    const r = await bootstrapNow(false)
    createdDirs.value = r.createdDirs
    installHint.value = r.installHint
    await loadConfig()
    await refreshDeps()
    if (!r.ok) {
      fail(`初始化未完全成功：${[...r.failed, ...r.missingDeps].join('；') || '未知原因'}`)
    } else {
      ok(`初始化完成：新建 ${r.createdDirs.length} 个目录，下载 ${r.downloaded.length} 项`)
    }
  })
}

async function copyHint() {
  await navigator.clipboard?.writeText(installHint.value)
  ok('命令已复制')
}

function goSource() {
  void router.push({ name: 'source' })
}

onMounted(async () => {
  await run(refreshDeps)
})
</script>

<style scoped>
.steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.step {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 6px;
  border: 1px solid var(--stroke-divider);
}

.step-idx {
  width: 26px;
  height: 26px;
  flex: 0 0 26px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  background: var(--subtle-secondary);
}

.step-body {
  flex: 1;
}

.step-title {
  font-size: 13.5px;
  font-weight: 600;
}

.step-mark {
  font-size: 16px;
  font-weight: 700;
}

.step-done .step-idx {
  background: var(--SystemFillColorSuccessBackgroundBrush);
  color: var(--SystemFillColorSuccessBrush);
}
.step-done .step-mark {
  color: var(--SystemFillColorSuccessBrush);
}

.step-fail .step-idx {
  background: var(--SystemFillColorCriticalBackgroundBrush);
  color: var(--SystemFillColorCriticalBrush);
}
.step-fail .step-mark {
  color: var(--SystemFillColorCriticalBrush);
}

.step-todo .step-mark {
  color: var(--text-disabled);
}
</style>
