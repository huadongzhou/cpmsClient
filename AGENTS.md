# CPMS Client Agent 说明

## 1. 项目定位

CPMS Client 当前是一个基于 Vue 3、TypeScript、Vite+ 和 Tauri 2 的桌面客户端工程。

项目目前处于 Tauri + Vue 基础架构搭建阶段：前端已具备应用壳层、路由、状态管理、HTTP / Tauri IPC 封装和基础页面入口；原生层保留一个 Rust 示例 command，并已切换为统一 `CommandResult` 返回结构；同时启用了 `tauri-plugin-opener`。

## 2. 当前技术栈

- 前端框架：Vue 3，使用 `<script setup lang="ts">`
- 类型系统：TypeScript
- 构建工具：Vite+，通过 `vp` 命令驱动
- 桌面容器：Tauri 2
- Rust 原生层：Rust 2021 edition
- 包管理：pnpm，当前 `packageManager` 为 `pnpm@11.5.0`
- Tauri 插件：`tauri-plugin-opener`

## 3. 项目目录架构

目录结构只保留对开发和维护有指导意义的部分。当前未落地但符合 CPMS 客户端演进方向的目录，可以按需要逐步补齐。`node_modules/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/` 等生成目录不纳入核心架构。

```text
cpmsClient/
  AGENTS.md                         # Agent 工作说明和项目约定
  README.md                         # 项目基础说明
  package.json                      # 前端依赖、脚本和 packageManager
  vite.config.ts                    # Vite+ / Vite 配置
  uno.config.ts                     # UnoCSS 配置

  public/                           # 不参与编译处理的静态资源

  src/                              # Web 前端源码
    main.ts                         # Vue 应用挂载入口
    App.vue                         # 应用根组件，承载应用壳层
    assets/                         # 图片、样式、字体等前端资源
    types/                          # 类型定义；按 app、common、service、task 分类

    router/                         # 前端路由；当前已包含工作台、服务、任务、设置、诊断入口
    stores/                         # 状态管理；应用、用户、网络、服务状态、任务状态等
    api/                            # 通信封装；按 http、request、cpms、localService、tauri、desktop 分类
    views/                          # 页面视图；工作台、服务、任务、设置、诊断等
    components/                     # 可复用组件；应用壳层、状态标识、错误提示等
    composables/                    # 组合式逻辑；错误上报、命令执行等

  src-tauri/                        # Tauri / Rust 原生层
    tauri.conf.json                 # Tauri 应用、窗口、打包配置
    Cargo.toml                      # Rust crate 配置
    capabilities/                   # Tauri 权限边界配置
    icons/                          # 应用图标资源

    src/                            # Rust 源码
      main.rs                       # 原生入口
      lib.rs                        # Tauri builder、插件注册、command 注册
      commands/                     # 对前端暴露的 Tauri command 入口
      services/                     # 本地服务控制、脚本执行、配置、日志、更新等能力
      infra/                        # 数据库、文件、日志、进程等基础设施封装
      security/                     # 白名单、参数校验、路径校验、权限判断
      platform/                     # Windows / Linux / macOS 差异封装
      models/                       # Rust 领域模型、DTO 和统一返回结构

    migrations/                     # 本地数据库迁移脚本
```

## 4. 通信方式与实现状态

状态说明：

- 已实现：当前代码中已有可运行实现。
- 部分实现：已有基础配置或示例，但尚未形成业务能力。
- 未实现：`Agent.md` 中规划或约定的能力，当前代码中尚未落地。

| 通信链路 | 通信方式 | 用途 | 当前状态 | 当前依据 |
| --- | --- | --- | --- | --- |
| Web UI -> Rust Core | Tauri `invoke` 调用 Rust command | 前端请求本地原生能力，如服务控制、脚本执行、配置读取 | 部分实现 | `src/api/tauri/client.ts` 已封装 `invokeCommand` / `unwrapCommand`，`src/api/tauri/commands.ts` 已集中声明 command 调用；`src-tauri/src/lib.rs` 已注册 `greet` 示例 command |
| Rust Core -> Web UI | Tauri event `emit` / 前端 `listen` | 原生层向前端推送任务状态、脚本日志、服务状态变化 | 未实现 | 当前未看到 `emit` / `listen` 事件封装 |
| Web UI -> CPMS Server | HTTP / HTTPS API | 访问 CPMS 后端业务接口 | 部分实现 | `src/api/http/client.ts` 已提供 fetch 请求封装，`src/api/cpms/client.ts` 已提供 CPMS API 客户端入口；尚未落地具体业务接口 |
| Web UI / Rust Core -> Local Service | 本地 HTTP 接口 | 访问本机 agent、健康检查或本地服务接口 | 部分实现 | 前端已在 `src/stores/app.ts` 维护 `localServiceUrl` 配置；尚未实现本地服务健康检查 |
| Rust Core -> Web UI | 统一结构化返回 `CommandResult` | 统一 command 成功、失败、日志和业务数据返回格式 | 已实现 | `src-tauri/src/models.rs` 已定义 `CommandResult<T>`；`greet` 示例 command 已返回统一结构；前端 `src/types/common/result.ts` 已定义对应类型 |

## 5. 客户端能力与实现状态

### 5.1 桌面能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 独立桌面窗口 | 已实现 | `tauri.conf.json` 已配置主窗口 `cpmsClient` |
| 外部链接打开 | 已实现 | 已注册 `tauri-plugin-opener` 并配置权限 |
| 系统托盘 | 未实现 | 未看到 tray 配置或插件 |
| 单实例运行 | 未实现 | 未看到 single-instance 插件或相关配置 |
| 开机自启动 | 未实现 | 未看到 autostart 插件或配置 |
| 桌面级通知子窗口 | 已实现 | 不使用系统级 notification 插件或权限；主窗口通过 `src/api/desktop/notification.ts` 创建 `notification` Tauri 子窗口，`src/components/common/DesktopNotificationHost.vue` 在桌面右下角显示通知 |
| 原生菜单 | 未实现 | 未看到菜单配置或 Rust 菜单构建 |
| 窗口最小化 / 最大化 / 置顶 | 未实现 | 未看到窗口控制 API 封装 |
| 隐藏到托盘 | 未实现 | 依赖系统托盘能力，当前未实现 |
| 国产系统 / Linux 打包 | 已配置 | `tauri.conf.json` 已配置 `deb`、`rpm`、`appimage` 目标，支持统信UOS、麒麟、方德等国产 Linux 发行版；需在对应平台或容器内执行构建 |
| 自动更新 | 未实现 | 未看到 updater 插件或更新配置 |

### 5.2 业务访问能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| CPMS 业务接口访问 | 部分实现 | 当前已有 `src/api/http/client.ts` 和 `src/api/cpms/client.ts` 请求入口，尚未实现具体业务 API |
| 登录态维护 | 部分实现 | 当前已有 `src/stores/user.ts` 维护 token 和用户显示名，尚未实现登录页面和接口 |
| 接口错误处理 | 部分实现 | 当前已有 `HttpError`、`CommandInvokeError` 和应用错误 store |
| 操作日志记录 | 未实现 | 当前没有日志服务或前端日志模块 |
| 异常状态提示 | 部分实现 | 当前已有 `src/components/common/ErrorNotice.vue` 和 `src/stores/app.ts` 错误队列 |
| 网络状态提示 | 部分实现 | 当前已有 `src/stores/network.ts` 和工作台 / 诊断页网络状态展示 |

### 5.3 本地数据能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 本地配置存储 | 部分实现 | 当前 `src/stores/app.ts` 使用 localStorage 保存客户端基础配置 |
| 用户偏好设置 | 部分实现 | 当前设置页已包含 API 地址、本地服务地址和日志级别 |
| 服务状态缓存 | 部分实现 | 当前已有服务状态 store，尚未接入真实服务检查 |
| 操作记录缓存 | 未实现 | 当前没有缓存或存储层 |
| 本地数据库存储 | 未实现 | 当前没有 SQLite 或数据库模块 |
| 配置导入导出 | 未实现 | 当前没有导入导出 API |

### 5.4 日志、诊断与安全能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 前端错误日志 | 部分实现 | 当前 `src/main.ts` 捕获 Vue runtime error 和 unhandled rejection，并写入应用错误队列 |
| 原生层运行日志 | 未实现 | 当前没有 Rust 日志服务 |
| 脚本执行日志 | 未实现 | 当前没有脚本任务实现 |
| 服务控制日志 | 未实现 | 当前没有服务控制实现 |
| 接口请求日志 | 未实现 | 当前没有请求层 |
| 本地诊断信息导出 | 未实现 | 当前没有诊断导出 command |
| 日志滚动 | 未实现 | 当前没有日志文件管理 |
| Tauri command 最小暴露 | 部分实现 | 当前仅暴露 `greet` 示例 command；前端业务 command 已集中声明但 Rust 业务 command 尚未实现 |
| 参数校验 | 部分实现 | `greet` 已进行空值校验；服务名、脚本名、路径等业务参数校验尚未实现 |
| 路径校验 / 敏感信息脱敏 / 配置加密 / 权限检测 / 高风险审计 | 未实现 | 当前没有对应安全模块 |

## 6. 开发命令

项目使用 Vite+ 工具链。

```bash
vp install
vp dev
vp build
vp check
vp test
```

当前 `package.json` scripts：

```bash
pnpm dev       # vp dev
pnpm build     # vue-tsc --noEmit && vp build
pnpm preview   # vp preview
pnpm prepare   # vp config
pnpm fmt       # vp fmt
pnpm lint      # vp lint
pnpm tauri     # tauri
```

Tauri 开发和构建命令：

```bash
pnpm tauri dev
pnpm tauri build
```

打包构建命令（单包 / 平台包 / 全量）：

```bash
# 平台包命令（一次性构建该平台全部格式）
pnpm build:win          # Windows 平台：msi + nsis
pnpm build:linux        # Linux 平台：deb + rpm + appimage（国产系统通用）

# 单包命令（一次性构建单个格式）
pnpm build:win:msi      # Windows MSI 安装包
pnpm build:win:nsis     # Windows NSIS 安装包
pnpm build:linux:deb    # Linux Debian 包（统信UOS、银河麒麟桌面版、中科方德）
pnpm build:linux:rpm    # Linux RPM 包（部分麒麟服务器版）
pnpm build:linux:appimage # Linux AppImage 通用包

# 指定架构的 Linux 平台包（国产系统常用架构）
pnpm build:linux:x64    # x86_64 / amd64
pnpm build:linux:arm64  # aarch64 / arm64（飞腾、鲲鹏）

# 全量命令（一次性构建当前平台支持的全部格式）
pnpm build:all          # Windows 下构建 msi + nsis；Linux 下构建 deb + rpm + appimage
```

> **注意**：
> - 所有命令均已通过 `scripts/build.js` 做平台适配，确保在对应平台上**一次性执行成功**，不会因混入不支持的跨平台格式而报错。
> - `build:all` 会根据当前宿主平台自动过滤可用格式（Windows 只打 `msi`/`nsis`，Linux 只打 `deb`/`rpm`/`appimage`）。如需真正产出全平台安装包，请在 CI 多平台矩阵中分别执行。
> - `deb`/`rpm`/`appimage` 需在 Linux 环境（物理机、容器或 WSL）中执行；`msi`/`nsis` 需在 Windows 环境中执行。

环境诊断命令：

```bash
vp env doctor
```

## 7. Agent 工作约定

- 输出使用 UTF-8。
- 修改代码前先读取当前实现。
- 文档只描述当前项目已经存在的结构、配置和命令。
- 默认遵循 Vite+ 工作流，优先使用 `vp` 命令。
- 不回滚用户已有变更。
- 只做和当前任务相关的最小必要修改。
