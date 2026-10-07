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

> 应用目标平台是 Linux，因此**打包**需在 Linux 环境完成（开发机为 Windows/macOS 也可做前端调试与 Rust 静态检查）。

```bash
# 1. 安装依赖并构建前端
npm install
npm run dev          # 浏览器/窗口内调试前端（无 Rust 后端时给出友好提示）
npm run type-check   # vue-tsc 类型检查

# 2. 在 Linux 上打包
npm run tauri:build            # deb + AppImage + rpm
npm run tauri:build:deb        # 仅 deb
npm run tauri:build:appimage   # 仅 AppImage
```

前端与后端通过 Tauri IPC 通信；命令列表与类型定义在 `src/api/tauri.ts` 与 `src/types/index.ts`，并和 `src-tauri/src/model.rs` 一一对应（camelCase）。

### Windows 上做 Rust 静态检查

后端代码零平台相关分支，因此 Windows 下的 `cargo check` 是 Linux 的有效代理。
本机没有 MSVC，但 rustup 装了 `stable-x86_64-pc-windows-gnu`，配合 Qt 自带的 MinGW 即可：

```bash
export PATH="/d/Qt/Tools/mingw1310_64/bin:$HOME/.cargo/bin:$PATH"
cd src-tauri && cargo +stable-x86_64-pc-windows-gnu check
```

> `tauri-build` 在 Windows 上会要求 `src-tauri/icons/icon.ico`（Linux 不需要）。
> 缺失时用 `python scripts/make-ico.py` 从现有 PNG 生成。

### 提交前的自检脚本

`src/` 目录在 Windows（不区分大小写）上开发、在 Linux CI（严格区分大小写）上构建，
大小写不匹配会在本地通过、在 CI 失败。以下脚本用于提前拦截：

```bash
node scripts/audit-import-case.mjs        # 审计全部导入的大小写是否与磁盘一致
node scripts/check-capabilities.mjs <合法权限清单>   # 校验 Tauri capabilities 权限标识符
node scripts/gen-global-components.mjs    # 重新生成 WinUIonWeb 全局组件类型声明
```

`scripts/check-capabilities.mjs` 需要一份「合法权限全集」文本（每行一条）。
可从 Tauri 构建报错的 `expected one of ...` 列表提取，或直接跑一次构建让它报错。

## 云端构建（GitHub Actions）

`.github/workflows/build.yml` 在 `ubuntu-22.04` 上构建并产出 `.deb` / `.AppImage` / `.rpm`：

- 手动触发：仓库页面 **Actions → Build → Run workflow**，可填发布标签与打包格式
- 推送 `v*` 标签：自动构建并创建草稿 Release
- PR 到 `main` / `master`：仅做构建验证

> 产物依赖 `libwebkit2gtk-4.1`，适用于 **Ubuntu 22.04 及以上**；
> 20.04 官方源没有 webkit2gtk-4.1，无法直接运行。

## 已知限制

- vendor 目录 `src/winuionweb/` 中，有 15 个文件在文件级关闭了类型检查
  （`// @ts-nocheck`）。该控件库大量使用 XAML 风格动态属性，设计上不做严格类型约束；
  逐个修类型收益低且上游更新会冲突。标记由 `scripts/mark-vendor-ts-nocheck.mjs`
  统一维护，可用 `--remove` 撤销。
  **本项目自身的代码类型检查是干净的（`npm run type-check` 零错误）。**
- 镜像「连通性测试」后端未单独暴露，通过实际下载时观察效果验证。

详见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。
