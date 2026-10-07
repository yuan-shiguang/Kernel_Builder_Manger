# Kernel Builder Manager

基于 **Tauri 2.0 + Vue 3 + WinUIonWeb** 的 Android 内核构建管理器，运行目标为 **Linux 桌面**（deb / AppImage / rpm）。

## 功能特性

- 内置 GitHub 镜像源（direct / ghproxy / jsdelivr 等），改善国内访问
- 内核源码拉取：默认 HTTPS，检测到 SSH Key 自动切换；支持浅克隆与完整克隆切换
- SUSFS 补丁：拉取与应用，失败给出 `.rej` 冲突片段与诊断建议
- 工具链自动识别：按内核 Linux 版本匹配 Clang / GCC，缺失一键下载
- KernelSU 集成：内置 9 个分支源，按上游仓库自带 `setup.sh` 集成，源码完整克隆
- 实时日志面板：构建/下载过程实时回显，错误自动高亮
- defconfig 选择/编辑：加载已有或自定义，自动备份 `.bak`
- GitHub Actions 远程编译：配置 Token 后触发工作流、监控运行、下载产物

## 技术栈

- 桌面外壳：Tauri 2.0（Rust）
- 前端：Vue 3 `<script setup>` + TypeScript + Vite 6 + vue-router 4
- UI 控件：WinUIonWeb（vendor 源码内置，见 `src/winuionweb/`）

## 目录结构

```
src/            前端（Vue 3）
src-tauri/      Rust 后端（Tauri 命令、工具链、Git、GitHub 等）
docs/           架构文档
```

## 开发 / 构建

> 应用目标平台是 Linux，因此打包需在 **Linux 环境** 完成（开发机为 Windows/macOS 也可做前端调试）。

```bash
# 1. 安装依赖并构建前端
npm install
npm run dev          # 浏览器/窗口内调试前端（无 Rust 后端时给出友好提示）

# 2. 在 Linux 上打包
npm run tauri:build            # deb + AppImage + rpm
npm run tauri:build:deb        # 仅 deb
npm run tauri:build:appimage   # 仅 AppImage
```

前端与后端通过 Tauri IPC 通信；命令列表与类型定义在 `src/api/tauri.ts` 与 `src/types/index.ts`，并和 `src-tauri/src/model.rs` 一一对应（camelCase）。

## 已知限制

- 在 Windows 开发机上无法直接 `cargo build`（缺少 MSVC 链接器 / MinGW / clang），需在 Linux 目标机编译打包。
- 镜像「连通性测试」后端未单独暴露，通过实际下载时观察效果验证。

详见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。
