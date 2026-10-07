# Kernel Builder Manager — 架构设计

> 运行在 Linux 桌面平台的 Android 内核构建管理器
> Tauri 2.0（Rust 后端） + Vue 3（前端） + WinUIonWeb（UI 组件库）

---

## 1. 目标与约束

| 项 | 说明 |
| --- | --- |
| 目标平台 | Linux 桌面（`.deb` / `.AppImage` / `.rpm`） |
| 开发平台 | Windows（MINGW64）/ Linux 均可，产物只面向 Linux |
| 前端 | Vue 3 `<script setup>` + TypeScript + Vite 6 + vue-router 4 |
| 后端 | Tauri 2.0 + Rust（`tokio` 异步运行时） |
| UI 库 | WinUIonWeb（**非 npm 包**，以 vendor 源码方式内置） |
| 本地依赖 | `git`、`patch`、`make`、`tar`、`unzip`、`zip`、`cpio`、`python3`、`bc`、`flex`、`bison` |

### 为什么 WinUIonWeb 用 vendor 方式

WinUIonWeb 未发布到 npm，其仓库是一个 Vue 3 演示工程。因此把它当作**源码依赖**引入：

```
WinUIonWeb/src/components/*.vue  ──复制──▶  src/winuionweb/components/
WinUIonWeb/src/styles/*          ──复制──▶  src/winuionweb/styles/
WinUIonWeb/src/assets/Fonts      ──复制──▶  src/winuionweb/assets/Fonts/
WinUIonWeb/src/utils/*           ──复制──▶  src/winuionweb/utils/
WinUIonWeb/src/shims-vue.d.ts    ──复制──▶  src/winuionweb/
```

`WinUIonWeb/` 原仓库保留在工程根目录以便后续升级比对（已从 Vite 监听中排除）。
接入层 `src/winuionweb/index.ts` 负责**全局注册**控件，包括 XAML 风格的点号名：

```ts
// 批量注册
const modules = import.meta.glob('./components/*.vue', { eager: true })
// 显式注册点号名（Vue 模板里写 <Expander.Header> / <Grid.RowDefinitions> 需要）
app.component('Grid.RowDefinitions', GridRowDefinitions)
app.component('Expander.Header', ExpanderHeader)
app.component('Button.Flyout', ButtonFlyout)
```

控件属性遵循 XAML 的 **PascalCase**：`Content`、`IsEnabled`、`ItemsSource`、
`DisplayMemberPath`、`Severity`、`IsExpanded`……`v-model` 绑定同名 camelCase
（`v-model:IsOn`、`v-model:Text`、`v-model:SelectedItem`）。

---

## 2. 目录结构

```
Kernel_Builder_Manger/
├── index.html                     # 入口，class="theme-dark"，含 CSP
├── vite.config.ts                 # 端口 1420；别名 @ / @winui；排除 src-tauri
├── package.json                   # 前端依赖与 tauri:* 脚本
├── scripts/gen_icons.py           # 生成 Tauri 图标
├── WinUIonWeb/                    # UI 库原仓库（只读参考，不参与构建）
├── src/                           # ── 前端 ──
│   ├── main.ts                    # 注册 WinUI → router → 订阅事件 → mount
│   ├── App.vue                    # NavigationView + 标题栏 + 日志停靠 + 状态栏
│   ├── router/index.ts            # 路由表 + navMenuItems + Segoe 图标字形
│   ├── api/tauri.ts               # ★ 唯一 IPC 出口：所有 invoke 与事件订阅
│   ├── types/index.ts             # ★ 与 model.rs 一一对应的 TS 类型
│   ├── stores/
│   │   ├── settings.ts            # 全局配置 + 主题 + 便捷 computed
│   │   └── log.ts                 # 日志环形缓冲 + 下载进度
│   ├── composables/useTask.ts     # busy / message / severity 状态机
│   ├── components/
│   │   ├── LogPanel.vue           # 日志面板（过滤 / 高亮 / 导出）
│   │   └── StatCard.vue           # 指标卡片
│   ├── views/                     # 10 个功能页面
│   │   ├── DashboardView.vue      # 概览
│   │   ├── BootstrapView.vue      # 初始化向导
│   │   ├── SourceView.vue         # 内核源码拉取 / 识别
│   │   ├── ToolchainView.vue      # 工具链匹配 / 下载
│   │   ├── SusfsView.vue          # SUSFS 补丁下载 / 应用 / 失败详情
│   │   ├── KsuView.vue            # KernelSU 全分支集成
│   │   ├── DefconfigView.vue      # defconfig 浏览 / 编辑 / 另存
│   │   ├── BuildView.vue          # 构建参数 / 脚本预览 / 产物
│   │   ├── ActionsView.vue        # GitHub Actions 远程编译
│   │   └── SettingsView.vue       # 镜像 / 工具链 / Token / 界面
│   ├── styles/app.css             # 应用级布局（建立在 theme.css 之上）
│   └── winuionweb/                # ── UI 库 vendor ──
└── src-tauri/                     # ── 后端 ──
    ├── Cargo.toml
    ├── tauri.conf.json            # bundle: deb / appimage / rpm；CSP 白名单
    ├── capabilities/default.json  # fs / dialog / shell / opener 权限
    ├── icons/
    └── src/
        ├── main.rs                # 入口
        ├── lib.rs                 # run()：插件 + 状态 + generate_handler!
        ├── model.rs               # ★ 全部数据结构（serde camelCase）
        ├── state.rs               # AppState：配置 / 运行中任务 / 日志序号
        ├── log.rs                 # 事件名常量 + 分级日志 + 关键字分类
        ├── config.rs              # config.json 读写 + 内置镜像合并
        ├── proc.rs                # 流式子进程执行 / 取消 / which
        ├── net.rs                 # HTTP 客户端 / 流式下载 / 解压
        ├── mirror.rs              # GitHub 镜像 URL 改写
        ├── git.rs                 # clone / fetch / checkout / SSH 探测
        ├── kernel.rs              # Makefile 版本解析 / defconfig 读写
        ├── toolchain.rs           # Clang/GCC 版本映射 / 下载 / 校验
        ├── susfs.rs               # SUSFS 拉取 / 打补丁 / 失败诊断
        ├── ksu.rs                 # KSU 分支注册表 / 完整克隆 / 集成
        ├── build.rs               # 构建脚本生成 / 执行 / 打包
        ├── github.rs              # Actions API
        ├── bootstrap.rs           # 首次初始化
        └── commands.rs            # ★ 40+ 个 #[tauri::command]
```

---

## 3. 后端分层

```
commands.rs             ← IPC 边界：参数校验、错误转字符串、调用服务层
    │
    ├── git.rs          ┐
    ├── kernel.rs       │
    ├── toolchain.rs    ├─ 业务服务层（纯函数／同步为主）
    ├── mirror.rs       │
    ├── susfs.rs        │
    ├── ksu.rs          ┘
    ├── build.rs        ┐
    ├── github.rs       ├─ 需要网络的异步层
    ├── bootstrap.rs    ┘
    │
    ├── proc.rs   net.rs        ← 基础设施：进程 / HTTP
    └── state.rs  log.rs  config.rs  model.rs
```

**关键约定**

1. `model.rs` 中所有结构体加 `#[serde(rename_all = "camelCase")]`，
   与 `src/types/index.ts` 字段一一对应，前端无需做任何字段名转换。
2. 命令统一返回 `R<T> = Result<T, String>`；错误信息是**给用户看的中文句子**，
   通常包含「原因 + 排查建议」，前端直接塞进 `InfoBar`。
3. 长任务（克隆 / 下载 / 编译 / 打补丁）通过**事件**回报进度，而不是返回值：
   - `log://line` → `LogLine { seq, task, stream, level, text, ts }`
   - `progress://download` → `ProgressEvent { id, label, received, total, percent, done, error }`

---

## 4. 前端数据流

```
        ┌──────────────── 用户操作 ────────────────┐
        ▼                                          │
   views/*.vue ──invoke──▶ api/tauri.ts ──IPC──▶ commands.rs
        │                                          │
        │                             emit(log://line) / (progress://download)
        │                                          │
        │                                          ▼
        └──────── stores/log.ts ◀──── listen ──────┘
                     │
                     ▼
             components/LogPanel.vue（过滤 + 高亮 + 导出）
```

- **配置**：`stores/settings.ts` 持有唯一 `config` ref。页面收集本地副本（避免
  直接改全局导致半保存），用户点「保存」时 `Object.assign` 回全局再 `persist()`。
- **日志**：环形缓冲，上限跟随 `ui.maxLogLines`（500–20000），超出从头裁剪。
- **提示**：`composables/useTask.ts` 提供 `run(fn, okMsg)`，自动管理 `busy` 与
  `InfoBar` 的 `severity`。业务级失败（如补丁冲突）用 `fail()` 显式覆盖。

---

## 5. 功能实现要点

### 5.1 GitHub 镜像（需求 1）

`MirrorConfig.mirrors` 内置 7 个源，三种改写策略：

| kind | 改写规则 | 适用 |
| --- | --- | --- |
| `direct` | 原样返回 | 兜底 |
| `prefix` | `prefix + 原 URL` | archive / raw / release 下载 |
| `jsdelivr` | `cdn.jsdelivr.net/gh/{owner}/{repo}@{ref}/{path}` | 仅 raw 单文件 |

`mirror::candidates(url, cfg)` 返回**按优先级排序的候选列表**：
当前镜像 → 其他已启用镜像（`autoFallback` 为真时）→ 直连。
`net::download_any` 依次尝试，任一成功即返回，全部失败则聚合错误。

`git clone` 无法走 URL 改写，因此克隆**不使用镜像**，而是在网络失败时提示用户
切换协议或使用 SSH。工具链下载走 `googlesource` 主源 + GitHub 镜像兜底。

### 5.2 源码拉取（需求 2）

`git::build_clone_url(input, protocol, ssh_status)`：

- `protocol = auto`：`ssh.has_key && ssh.github_host_configured` → SSH，否则 HTTPS
- 支持 `owner/repo`、完整 URL、`git@github.com:owner/repo.git` 三种输入

`git::clone` 组装参数：

```
git clone --progress
  [--depth N --single-branch]   # shallow = true 时
  [--recurse-submodules]
  <url> <dest>
```

`shallow` 默认 `true`，可在「设置 → 源码」或源码页关闭。关闭后走完整历史，
使 `git apply --3way` 与后续 `git revert` 可用。

`preview_clone_url` 命令让前端实时显示将要执行的地址，便于确认协议切换生效。

### 5.3 SUSFS 补丁（需求 3）

SUSFS 源码来自 `JackA1ltman/NonGKI_Kernel_Build_2nd`。

```
susfs_fetch   → full_clone=true 时完整 git 克隆；否则 codeload tarball + 解压
susfs_list    → 扫描 *.patch / *.diff，给出 size 与目标提示（hint）
susfs_apply   → 逐个应用，返回 PatchApplyResult[]
```

**补丁应用双策略回退**（`susfs::apply_patch_file`）：

```
1. git apply --3way --whitespace=nowarn  <patch>
       │ 失败（且非 --3way 不可用）
       ▼
2. patch -p1 --forward --no-backup-if-mismatch < <patch>
```

失败时的**失败详情生成链路**（满足「必须明确提示并展示失败详情」）：

```
收集 *.rej 内容
  → git apply --check 诊断输出
  → 生成 build_failure_detail：
        · 冲突文件列表
        · 逐条 .rej 片段
        · 4 条排查建议（内核版本不匹配 / 已打过同补丁 /
          源码非 git 仓库 / 内核为浅克隆导致缺少历史）
  → 作为 PatchApplyResult.detail 返回前端
  → 前端用 Expander 展开渲染 + 日志面板同步 log_error
```

### 5.4 工具链匹配（需求 4、5）

`toolchain::clang_for_kernel(v) / gcc_for_kernel(v)` 依据内核主次版本选择：

| 内核 | Clang | GCC |
| --- | --- | --- |
| 未知（兜底） | AOSP Clang r365631c | 4.9 |
| 4.4 / 4.9 | Clang 9（r353983c） | 4.4 / 4.9 |
| 4.14 | Clang 11（r399163b1） | 4.14 |
| 4.19 / 5.4 | Clang 12（r383902） | 4.19 / 5.4 |
| 5.10 | Clang 14（r416183b） | 5.10 |
| 5.15 / 6.1 | Clang 17（r450784e / r487747c） | 5.15 / 6.1 |

`toolchain_plan` 返回 `ToolchainPlan`，其中 `missing` 是**真正缺失**的组件列表，
概览页与构建页据此显示黄色警告并提供「一键补齐」。
`toolchain_install` 下载 AOSP 预编译包（googlesource）后校验 `bin/clang --version`。

### 5.5 构建（需求 6、7）

`build::generate_script(cfg, plan)` 生成 `build.sh`，包含：

```sh
export ARCH=arm64
export SUBARCH=arm64
export CC=<clang>            # 来自 ToolchainPlan
export CLANG_TRIPLE=aarch64-linux-gnu-
export CROSS_COMPILE=<gcc-aarch64>
export PATH=<clang>/bin:$PATH
make -j<jobs> O=<out> <defconfig>
make -j<jobs> O=<out> <target> [LTO=thin] [额外参数]
```

`build_preview` 让用户在运行前审阅脚本；`build_start` 流式执行，stdout/stderr
逐行推送到 `log://line`，`log::classify()` 把含 `error/fatal/failed` 的行标为
error 等级，前端 `LogPanel.highlight()` 再叠加 `<mark>` 高亮。

### 5.6 KernelSU 全分支（追加需求）

`ksu::PROVIDERS` 注册 7 个源：

| id | 仓库 | 说明 |
| --- | --- | --- |
| `kernelsu` | `tiann/KernelSU` | 官方，@tiann |
| `next` | `rifsxd/KernelSU-Next` | KernelSU-Next |
| `sukisu-ultra` | `ShirkNeko/SukiSU-Ultra` | SukiSU-Ultra |
| `bakasu` | `Baka-SU/BakaSU` | 即原 ReSukiSU，从 SukiSU-Ultra 分叉 |
| `rsuntk` | `rsuntk/KernelSU` | @rsuntk 分支 |
| `rsuntk-susfs` | `cyberc3dr/KernelSU` | rsuntk + SUSFS |
| `xxksu` | `backslashxx/KernelSU` | xxKSU |

两条硬约束（用户明确要求）：

1. **必须使用仓库自带的 `setup.sh`**
   `ksu::integrate` 中 `method = "setup"` 时执行 `kernel/setup.sh <git_ref>`。
   失败时才回退到 `integrate_manual`（复制 `kernel/` + 逐个打补丁）。

2. **必须完整拉取，不能浅克隆**
   `ksu::fetch_source` 调用 `git::clone_full`，源码中有显式注释：
   ```rust
   log_info(app, "ksu", &format!("完整克隆 {label}（{url}）—— 不使用 --depth，保留全部分支与历史"));
   git::clone_full(app, &url, &dir)?;
   ```
   每个仓库只在工作区保留**一份**完整克隆，切换分支用 `git checkout`，
   因此「分支列表」可以离线获取（`git branch -r`），无需每次打 GitHub API。

分支/标签获取顺序：本地仓库 `git branch -r` → GitHub API 兜底 → 内置默认分支。

### 5.7 GitHub Actions（需求 8）

`github.rs` 封装 REST API：

| 命令 | 端点 |
| --- | --- |
| `gh_list_workflows` | `GET /repos/{o}/{r}/actions/workflows` |
| `gh_dispatch` | `POST /repos/{o}/{r}/actions/workflows/{id}/dispatches` |
| `gh_list_runs` | `GET /repos/{o}/{r}/actions/runs` |
| `gh_list_artifacts` | `GET /repos/{o}/{r}/actions/runs/{id}/artifacts` |
| `gh_download_artifact` | `GET .../artifacts/{id}/zip`（302 跟随 + 解压） |

Token 存在 `config.json` 的 `github.token`，请求头 `Authorization: Bearer <token>`。
`api_base` 可改为企业版或代理地址。

---

## 6. 事件协议

### `log://line`

```jsonc
{
  "seq": 1234,               // 后端单调递增，前端用作 :key
  "task": "build",           // 模块标识，见 stores/log.ts 的 TASK_LABELS
  "stream": "stdout",        // stdout | stderr | system
  "level": "error",          // info | warn | error | success
  "text": "make[1]: *** No rule to make target 'Image'",
  "ts": 1759757000000
}
```

`level` 由后端 `log::classify()` 判定（关键字表 + 来源流）；
前端 `LogPanel.highlight()` 再做一次正则高亮，两层互补。

### `progress://download`

```jsonc
{
  "id": "toolchain-clang-r416183b",
  "label": "Clang r416183b",
  "received": 314572800,
  "total": 1073741824,
  "percent": 29.3,
  "done": false,
  "error": null
}
```

`done = true` 且 `error != null` 时，`stores/log.ts` 会自动补一条 error 日志，
保证下载失败不会被静默吞掉。

---

## 7. 安全与权限

| 项 | 配置 |
| --- | --- |
| CSP | `default-src 'self'`；`connect-src` 白名单 `api.github.com`、`*.github.com`、`*.githubusercontent.com`、`*.googlesource.com`、`ipc:` |
| fs 权限 | `capabilities/default.json` 限定 `$HOME`、`$DESKTOP`、`$DOCUMENT`、`$DOWNLOAD`、`$APPDATA` 递归 |
| 子进程 | 全部经 `proc.rs` 统一入口，参数以 `Vec<String>` 传递（不经 shell），避免注入 |
| Token | 仅存本地 `config.json`，不写入日志（日志层对含 token 的行做过滤） |

---

## 8. 构建与运行

```bash
# 前端依赖
npm install

# 开发（需要 Linux + WebKitGTK；Windows 上可只跑前端预览）
npm run tauri:dev

# 仅前端（浏览器预览，IPC 调用会给出明确提示）
npm run dev

# 类型检查
npm run type-check

# 打包（在 Linux 上执行）
npm run tauri:build:deb
npm run tauri:build:appimage
npm run tauri:build          # 全部 targets：deb + appimage + rpm
```

Linux 构建前置依赖（Debian/Ubuntu）：

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf build-essential curl wget file \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev
```

### 平台说明

- 目标产物只针对 **Linux**。在 Windows 上 `cargo check`/`tauri build` 需要
  MSVC 或 MinGW 链接器，仅用于本机语法校验，不代表最终运行环境。
- 内核编译本身需要 Linux 主机（`make`、`patch`、`ccache` 等），
  Windows 上可用「远程编译」页把构建交给 GitHub Actions。

---

## 9. 扩展点

| 想加什么 | 改哪里 |
| --- | --- |
| 新的 GitHub 镜像 | `model.rs::Mirror::builtin()` 追加一项，无需改前端 |
| 新的 KSU 分支源 | `ksu.rs::PROVIDERS` 追加 `KsuProvider` |
| 新的内核版本映射 | `toolchain.rs::clang_for_kernel / gcc_for_kernel` |
| 新的构建参数 | `BuildConfig` 加字段 + `build.rs::generate_script` |
| 新的 UI 页面 | `views/` 新建 `.vue` + `router/index.ts` 加路由（菜单自动出现） |
