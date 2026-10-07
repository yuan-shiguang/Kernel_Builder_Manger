<template>
  <div>
    <h1 class="page-title">SUSFS 补丁</h1>
    <p class="page-subtitle">
      从 <span class="mono">{{ config?.susfs.repo }}</span> 拉取补丁并应用到内核源码；
      应用失败时会给出冲突文件与完整诊断。
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

    <!-- 仓库设置 -->
    <div class="card">
      <div class="section-title">补丁仓库</div>
      <div class="row">
        <TextBox v-model:Text="susfsRepo" Header="仓库" Width="320" />
        <TextBox v-model:Text="susfsBranch" Header="分支" Width="160" />
        <ComboBox
          :ItemsSource="methodOptions"
          DisplayMemberPath="Content"
          v-model:SelectedItem="selMethod"
          Header="应用方式"
          Width="200"
        />
        <ToggleSwitch
          v-model:IsOn="keepRejects"
          Header="冲突文件"
          OnContent="保留 .rej"
          OffContent="清除 .rej"
        />
      </div>
      <div class="row" style="margin-top: 12px">
        <Button Content="拉取补丁仓库" :IsEnabled="!busy" @Click="doFetch" />
        <Button Content="刷新补丁列表" @Click="loadPatches" />
      </div>
    </div>

    <!-- 补丁列表 -->
    <div class="card section">
      <div class="section-title">
        补丁文件（{{ patches.length }}）
        <span class="spacer" />
        <ToggleSwitch v-model:IsOn="stopOnError" Header="遇错停止" OnContent="停止" OffContent="继续" />
      </div>

      <p v-if="!patches.length" class="muted">
        尚未拉取补丁。点击「拉取补丁仓库」从 NonGKI_Kernel_Build_2nd 获取。
      </p>

      <div v-else>
        <div class="row" style="margin-bottom: 8px">
          <Button Content="全选" @Click="selectAll" />
          <Button Content="清空选择" @Click="clearSel" />
          <span class="spacer" />
          <Button Content="应用选中补丁" :IsEnabled="!busy && selected.length > 0" @Click="doApply" />
        </div>

        <table class="kv-table">
          <thead>
            <tr>
              <th style="width: 36px"></th>
              <th>补丁</th>
              <th style="width: 90px">大小</th>
              <th>目标提示</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in patches" :key="p.path">
              <td>
                <input v-model="selected" type="checkbox" :value="p.path" />
              </td>
              <td class="mono">{{ p.name }}</td>
              <td>{{ formatSize(p.size) }}</td>
              <td class="muted mono">{{ p.hint }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 应用结果 -->
    <div v-if="results.length" class="section">
      <div class="section-title">应用结果</div>
      <div
        v-for="(r, i) in results"
        :key="i"
        class="card"
        style="margin-bottom: 10px; border-color: transparent"
      >
        <div class="row-between">
          <div class="row">
            <span class="badge" :class="r.ok ? 'badge-ok' : 'badge-err'">
              {{ r.ok ? '成功' : '失败' }}
            </span>
            <span class="mono">{{ r.failed[0] || r.applied[0] || '' }}</span>
          </div>
          <span class="muted mono">{{ r.method }} · 退出码 {{ r.exitCode }}</span>
        </div>

        <p v-if="!r.ok" style="margin: 8px 0">{{ r.message }}</p>

        <Expander v-if="!r.ok" :IsExpanded="true">
          <Expander.Header>
            <TextBlock Text="查看失败详情与诊断" />
          </Expander.Header>
          <div class="code-block" style="margin-bottom: 10px">{{ r.detail }}</div>

          <div v-if="r.rejects.length">
            <div class="muted" style="margin-bottom: 6px">冲突片段（.rej）：</div>
            <Expander v-for="rej in r.rejects" :key="rej.file" :IsExpanded="false">
              <Expander.Header>
                <TextBlock :Text="rej.file" />
              </Expander.Header>
              <div class="code-block">{{ rej.content }}</div>
            </Expander>
          </div>
        </Expander>
      </div>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">补丁日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import { formatSize, susfsApply, susfsFetch, susfsList } from '@/api/tauri'
import { config, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const susfsRepo = ref('')
const susfsBranch = ref('main')
const keepRejects = ref(true)
const stopOnError = ref(true)

const methodOptions = [
  { Tag: 'auto', Content: '自动（git → patch）' },
  { Tag: 'git', Content: '仅 git apply' },
  { Tag: 'patch', Content: '仅 patch -p1' },
]
const selMethod = ref(methodOptions[0])

const patches = ref<T.PatchFileInfo[]>([])
const selected = ref<string[]>([])
const results = ref<T.PatchApplyResult[]>([])

const failedCount = computed(() => results.value.filter((r) => !r.ok).length)

async function doFetch() {
  if (config.value) {
    config.value.susfs.repo = susfsRepo.value
    config.value.susfs.branch = susfsBranch.value
    await persist()
  }
  await run(async () => {
    await susfsFetch()
    await loadPatches()
  }, '补丁仓库已更新')
}

async function loadPatches() {
  await run(async () => {
    patches.value = await susfsList()
  })
}

async function doApply() {
  if (!config.value?.general.kernelDir) return fail('请先选择内核目录')
  if (!selected.value.length) return fail('请至少选择一个补丁')
  await run(async () => {
    results.value = await susfsApply(selected.value, stopOnError.value)
    const failed = results.value.filter((r) => !r.ok)
    if (failed.length) {
      fail(`${failed.length} 个补丁应用失败，请展开下方「失败详情」查看冲突与诊断`)
    } else {
      ok(`全部 ${results.value.length} 个补丁应用成功`)
    }
  })
}

function selectAll() {
  selected.value = patches.value.map((p) => p.path)
}
function clearSel() {
  selected.value = []
}

watch(selMethod, async (v) => {
  if (config.value) {
    config.value.susfs.applyMethod = v?.Tag ?? 'auto'
    await persist()
  }
})
watch(keepRejects, async (v) => {
  if (config.value) {
    config.value.susfs.keepRejects = v
    await persist()
  }
})

onMounted(async () => {
  if (config.value) {
    susfsRepo.value = config.value.susfs.repo
    susfsBranch.value = config.value.susfs.branch
    keepRejects.value = config.value.susfs.keepRejects
    selMethod.value =
      methodOptions.find((m) => m.Tag === config.value!.susfs.applyMethod) ?? methodOptions[0]
  }
  await loadPatches()
})
</script>
