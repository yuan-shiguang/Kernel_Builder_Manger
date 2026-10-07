<template>
  <div>
    <h1 class="page-title">远程编译</h1>
    <p class="page-subtitle">
      配置 GitHub Token 后，可直接触发仓库的 GitHub Actions 工作流进行云端编译，并回传产物。
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

    <!-- 配置 -->
    <div class="card">
      <div class="section-title">GitHub 配置</div>
      <div class="row">
        <TextBox
          v-model:Text="cfg.token"
          Header="Token（classic，需 workflow 权限）"
          PlaceholderText="ghp_xxxxxxxx"
          Width="340"
        />
        <TextBox v-model:Text="cfg.owner" Header="Owner" PlaceholderText="your-name" Width="180" />
        <TextBox v-model:Text="cfg.repo" Header="Repo" PlaceholderText="your-kernel" Width="220" />
        <TextBox v-model:Text="cfg.gitRef" Header="Git Ref" PlaceholderText="main" Width="160" />
      </div>
      <div class="row" style="margin-top: 12px">
        <Button Content="保存配置" @Click="saveGh" />
        <Button Content="列出工作流" @Click="loadWorkflows" />
        <Button Content="列出最近运行" @Click="loadRuns" />
      </div>
    </div>

    <!-- 工作流 -->
    <div v-if="workflows.length" class="card section">
      <div class="section-title">工作流（{{ workflows.length }}）</div>
      <table class="kv-table">
        <thead>
          <tr><th>名称</th><th>路径</th><th>状态</th><th>操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="w in workflows" :key="w.id">
            <td>{{ w.name }}</td>
            <td class="mono">{{ w.path }}</td>
            <td>
              <span class="badge" :class="w.state === 'active' ? 'badge-ok' : 'badge-mute'">{{ w.state }}</span>
            </td>
            <td>
              <Button Content="触发" @Click="dispatch(w.id)" />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 触发参数 -->
    <div class="card section">
      <div class="section-title">手动触发参数（JSON）</div>
      <TextBox v-model:Text="inputsJson" PlaceholderText='{"ARCH":"arm64"}' Width="480" />
      <Button Content="用当前配置触发" :IsEnabled="!busy && !!selWorkflowId" @Click="dispatch(selWorkflowId)" />
      <p class="muted" style="margin-top: 6px">选中的 Workflow ID：{{ selWorkflowId || '未选择' }}</p>
    </div>

    <!-- 运行记录 -->
    <div v-if="runs.length" class="card section">
      <div class="section-title">最近运行（{{ runs.length }}）</div>
      <table class="kv-table">
        <thead>
          <tr>
            <th>标题</th><th>分支</th><th>状态</th><th>结论</th><th>时间</th><th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in runs" :key="r.id">
            <td class="mono">{{ r.displayTitle }}</td>
            <td>{{ r.headBranch }}</td>
            <td>
              <span class="badge" :class="runBadge(r)">{{ r.status }}</span>
            </td>
            <td>
              <span class="badge" :class="conclusionBadge(r)">{{ r.conclusion || '—' }}</span>
            </td>
            <td class="muted">{{ r.createdAt }}</td>
            <td>
              <Button Content="产物" @Click="showArtifacts(r.id)" />
              <Button Content="取消" @Click="cancelRun(r.id)" />
              <Button Content="打开" @Click="openUrl(r.htmlUrl)" />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Artifacts -->
    <div v-if="artifacts.length" class="card section">
      <div class="section-title">Artifacts（Run {{ activeRunId }}）</div>
      <table class="kv-table">
        <thead>
          <tr><th>名称</th><th>大小</th><th>状态</th><th>操作</th></tr>
        </thead>
        <tbody>
          <tr v-for="a in artifacts" :key="a.id">
            <td class="mono">{{ a.name }}</td>
            <td>{{ formatSize(a.size) }}</td>
            <td>
              <span class="badge" :class="a.expired ? 'badge-err' : 'badge-ok'">
                {{ a.expired ? '已过期' : '可用' }}
              </span>
            </td>
            <td>
              <Button Content="下载" :IsEnabled="!a.expired" @Click="downloadArtifact(a)" />
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">Actions 日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import {
  formatSize,
  ghCancelRun,
  ghDispatch,
  ghDownloadArtifact,
  ghListArtifacts,
  ghListRuns,
  ghListWorkflows,
} from '@/api/tauri'
import { config, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const cfg = ref({
  token: config.value?.github.token ?? '',
  owner: config.value?.github.owner ?? '',
  repo: config.value?.github.repo ?? '',
  gitRef: config.value?.github.gitRef ?? 'main',
})

const workflows = ref<T.GhWorkflow[]>([])
const runs = ref<T.GhRun[]>([])
const artifacts = ref<T.GhArtifact[]>([])
const activeRunId = ref<number | null>(null)
const inputsJson = ref('')
const selWorkflowId = ref('')

async function saveGh() {
  if (!config.value) return
  config.value.github.token = cfg.value.token
  config.value.github.owner = cfg.value.owner
  config.value.github.repo = cfg.value.repo
  config.value.github.gitRef = cfg.value.gitRef
  await persist()
  ok('GitHub 配置已保存')
}

async function loadWorkflows() {
  await run(async () => {
    workflows.value = await ghListWorkflows()
    if (workflows.value.length) {
      selWorkflowId.value = String(workflows.value[0].id)
    }
  })
}

async function loadRuns() {
  await run(async () => {
    runs.value = await ghListRuns()
  })
}

async function dispatch(wid: string | number) {
  const id = String(wid)
  let inputs: Record<string, unknown> = {}
  try {
    inputs = inputsJson.value ? JSON.parse(inputsJson.value) : {}
  } catch {
    return fail('触发参数 JSON 格式错误')
  }
  await run(async () => {
    await ghDispatch(id, cfg.value.gitRef, inputs)
  }, '已触发工作流')
}

async function showArtifacts(runId: number) {
  activeRunId.value = runId
  await run(async () => {
    artifacts.value = await ghListArtifacts(runId)
  })
}

async function downloadArtifact(a: T.GhArtifact) {
  await run(async () => {
    const path = await ghDownloadArtifact(a)
    ok(`已下载：${path}`)
  })
}

async function cancelRun(id: number) {
  await run(async () => {
    const r = await ghCancelRun(id)
    if (r.ok) ok(r.message)
    else fail(r.message)
  })
}

function openUrl(url: string) {
  window.open(url, '_blank')
}

function runBadge(r: T.GhRun) {
  if (r.status === 'completed') return 'badge-ok'
  if (r.status === 'in_progress') return 'badge-warn'
  return 'badge-mute'
}
function conclusionBadge(r: T.GhRun) {
  if (r.conclusion === 'success') return 'badge-ok'
  if (r.conclusion === 'failure') return 'badge-err'
  if (r.conclusion === 'cancelled') return 'badge-warn'
  return 'badge-mute'
}
</script>
