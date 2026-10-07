<template>
  <div>
    <h1 class="page-title">defconfig 配置</h1>
    <p class="page-subtitle">
      加载现有 defconfig 进行查看与修改；保存时自动备份原文件（<span class="mono">*.bak</span>）。
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
      <!-- 选择区 -->
      <div class="card">
        <div class="section-title">选择 defconfig</div>

        <p v-if="!kernelDir" class="muted">尚未选择内核目录，请先到「内核源码」页拉取或识别源码。</p>

        <template v-else>
          <ComboBox
            :ItemsSource="archOptions"
            DisplayMemberPath="Content"
            v-model:SelectedItem="selArch"
            Header="目标架构"
            Width="240"
          />
          <div class="row" style="margin-top: 12px">
            <Button Content="列出 defconfig" :IsEnabled="!busy" @Click="loadList" />
          </div>

          <p v-if="names.length" class="muted" style="margin: 12px 0 6px">
            共 {{ names.length }} 个（点击加载到编辑器）：
          </p>
          <div class="def-list">
            <button
              v-for="n in names"
              :key="n"
              class="def-chip"
              :class="{ active: n === activeName }"
              @click="openDef(n)"
            >
              {{ n }}
            </button>
          </div>

          <div class="row" style="margin-top: 14px">
            <TextBox
              v-model:Text="newName"
              Header="新配置名"
              PlaceholderText="my_kernel_defconfig"
              Width="280"
            />
            <Button Content="另存为新配置" :IsEnabled="!!edited && !busy" @Click="saveAs" />
          </div>
        </template>
      </div>

      <!-- 编辑器 -->
      <div class="card">
        <div class="section-title">
          编辑器
          <span v-if="activeName" class="muted mono">· {{ activeName }}</span>
          <span class="spacer" />
          <ToggleSwitch v-model:IsOn="alsoSelect" Header="保存后设为当前" OnContent="是" OffContent="否" />
        </div>

        <p v-if="!activeName && !edited" class="muted">
          从左侧选择一个 defconfig，或在下方直接编辑后「另存为新配置」。
        </p>

        <label class="field-label">配置内容</label>
        <textarea
          v-model="edited"
          class="code-area"
          spellcheck="false"
          placeholder="# 每行一个配置，例如：
CONFIG_LOCALVERSION=&quot;-mykernel&quot;
CONFIG_KPROBES=y"
        ></textarea>

        <div class="row" style="margin-top: 12px">
          <Button Content="保存覆盖" :IsEnabled="!!activeName && !busy" @Click="saveOverwrite" />
          <Button Content="刷新列表" @Click="loadList" />
        </div>

        <p class="muted" style="margin-top: 10px">
          当前构建使用的 defconfig：<span class="mono">{{ config?.build.defconfig || '未设置' }}</span>
        </p>
      </div>
    </div>

    <div class="section" style="height: 300px">
      <div class="section-title">配置日志</div>
      <LogPanel />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'

import LogPanel from '@/components/LogPanel.vue'
import { useTask } from '@/composables/useTask'
import { listDefconfigs, readDefconfig, scanKernel, writeDefconfig } from '@/api/tauri'
import { config, kernelDir, persist } from '@/stores/settings'
import type * as T from '@/types'

const { busy, message, severity, run, clear, ok, fail } = useTask()

const arch = ref('arm64')
const names = ref<string[]>([])
const activeName = ref('')
const edited = ref('')
const newName = ref('')
const alsoSelect = ref(true)

const archOptions = [
  { Tag: 'arm64', Content: 'arm64 (aarch64)' },
  { Tag: 'arm', Content: 'arm (arm32)' },
  { Tag: 'x86_64', Content: 'x86_64' },
  { Tag: 'x86', Content: 'x86 (i386)' },
  { Tag: 'riscv', Content: 'riscv (riscv64)' },
]
const selArch = ref(archOptions[0])

watch(selArch, (v) => {
  arch.value = v?.Tag ?? 'arm64'
})

async function loadList() {
  if (!kernelDir.value) return fail('尚未选择内核目录')
  await run(async () => {
    names.value = await listDefconfigs(kernelDir.value, arch.value)
  })
}

async function openDef(name: string) {
  if (!kernelDir.value) return
  await run(async () => {
    edited.value = await readDefconfig(kernelDir.value, arch.value, name)
    activeName.value = name
    newName.value = name
  })
}

async function saveOverwrite() {
  if (!kernelDir.value) return fail('尚未选择内核目录')
  if (!activeName.value) return fail('没有可覆盖的目标')
  await run(async () => {
    await writeDefconfig(kernelDir.value, arch.value, activeName.value, edited.value, alsoSelect.value)
    if (alsoSelect.value && config.value) {
      config.value.build.defconfig = activeName.value
      await persist()
    }
  }, `已保存 ${activeName.value}（原文件已备份为 .bak）`)
}

async function saveAs() {
  if (!kernelDir.value) return fail('尚未选择内核目录')
  const name = (newName.value || '').trim()
  if (!name) return fail('请填写新配置名')
  await run(async () => {
    await writeDefconfig(kernelDir.value, arch.value, name, edited.value, alsoSelect.value)
    if (alsoSelect.value && config.value) {
      config.value.build.defconfig = name
      await persist()
    }
    await loadList()
  }, `已另存为 ${name}`)
}

onMounted(async () => {
  if (kernelDir.value) {
    try {
      const k: T.KernelInfo = await scanKernel(kernelDir.value)
      if (k.valid && k.archs.length) {
        const hit = archOptions.find((a) => k.archs.includes(a.Tag))
        if (hit) selArch.value = hit
      }
    } catch {
      /* ignore */
    }
    await loadList()
  }
})
</script>

<style scoped>
.def-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.def-chip {
  border: 1px solid var(--card-stroke, #3a3a3a);
  background: var(--card-bg, #2a2a2a);
  color: var(--text-primary, #fff);
  border-radius: 14px;
  padding: 4px 12px;
  font-size: 12.5px;
  cursor: pointer;
}

.def-chip:hover {
  border-color: var(--accent, #6b9fff);
}

.def-chip.active {
  background: var(--accent, #6b9fff);
  color: #fff;
  border-color: var(--accent, #6b9fff);
}

.field-label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin: 4px 0 6px;
}

.code-area {
  width: 100%;
  box-sizing: border-box;
  min-height: 320px;
  resize: vertical;
  background: var(--card-bg, #1e1e1e);
  color: var(--text-primary, #fff);
  border: 1px solid var(--card-stroke, #3a3a3a);
  border-radius: var(--app-radius, 8px);
  padding: 10px 12px;
  font-family: var(--mono-font, monospace);
  font-size: 12.5px;
  line-height: 1.55;
}
</style>
