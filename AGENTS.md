# CPMS Client Agent 说明

## 1. 项目定位

CPMS Client 当前是一个基于 Vue 3、TypeScript、Vite+ 和 Tauri 2 的桌面客户端工程。

项目目前处于 Tauri + Vue 初始骨架阶段：前端为单入口应用，原生层注册了一个 Rust 示例 command，并启用了 `tauri-plugin-opener`。

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
    App.vue                         # 当前示例页面，后续承载应用根组件
    assets/                         # 图片、样式、字体等前端资源
    types/                          # 自动生成类型和公共类型声明

    router/                         # 前端路由；按业务页面拆分
    stores/                         # 状态管理；应用、用户、服务状态等
    api/                            # 通信封装；CPMS HTTP 和 Tauri IPC
    views/                          # 页面视图；首页、工作台、任务、设置、诊断等
    components/                     # 可复用组件；布局、通用组件、服务组件等
    composables/                    # 组合式逻辑；服务状态、命令任务、系统事件等

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
| Web UI -> Rust Core | Tauri `invoke` 调用 Rust command | 前端请求本地原生能力，如服务控制、脚本执行、配置读取 | 部分实现 | `src/App.vue` 已调用 `invoke("greet", { name })`，`src-tauri/src/lib.rs` 已注册 `greet` command；当前仅为示例字符串返回 |
| Rust Core -> Web UI | Tauri event `emit` / 前端 `listen` | 原生层向前端推送任务状态、脚本日志、服务状态变化 | 未实现 | 当前未看到 `emit` / `listen` 事件封装 |
| Web UI -> CPMS Server | HTTP / HTTPS API | 访问 CPMS 后端业务接口 | 未实现 | 当前未看到 `src/api/http.ts`、`cpms.api.ts` 或请求封装 |
| Web UI / Rust Core -> Local Service | 本地 HTTP 接口 | 访问本机 agent、健康检查或本地服务接口 | 未实现 | 当前未看到本地 HTTP 客户端或服务健康检查实现 |
| Rust Core -> Web UI | 统一结构化返回 `CommandResult` | 统一 command 成功、失败、日志和业务数据返回格式 | 未实现 | 当前 `greet` 直接返回 `String`，尚未返回统一结果结构 |

## 5. 客户端能力与实现状态

### 5.1 桌面能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 独立桌面窗口 | 已实现 | `tauri.conf.json` 已配置主窗口 `cpmsClient` |
| 外部链接打开 | 已实现 | 已注册 `tauri-plugin-opener` 并配置权限 |
| 系统托盘 | 未实现 | 未看到 tray 配置或插件 |
| 单实例运行 | 未实现 | 未看到 single-instance 插件或相关配置 |
| 开机自启动 | 未实现 | 未看到 autostart 插件或配置 |
| 系统通知 | 未实现 | 未看到 notification 插件或权限 |
| 原生菜单 | 未实现 | 未看到菜单配置或 Rust 菜单构建 |
| 窗口最小化 / 最大化 / 置顶 | 未实现 | 未看到窗口控制 API 封装 |
| 隐藏到托盘 | 未实现 | 依赖系统托盘能力，当前未实现 |
| 自动更新 | 未实现 | 未看到 updater 插件或更新配置 |

### 5.2 业务访问能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| CPMS 业务接口访问 | 未实现 | 当前没有业务 API 封装 |
| 登录态维护 | 未实现 | 当前没有路由、store 或登录态模块 |
| 接口错误处理 | 未实现 | 当前没有统一请求层 |
| 操作日志记录 | 未实现 | 当前没有日志服务或前端日志模块 |
| 异常状态提示 | 未实现 | 当前仅有 Tauri 示例页面 |
| 网络状态提示 | 未实现 | 当前没有网络状态检测 |

### 5.3 本地服务控制能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 查询 / 启动 / 停止 / 重启服务 | 未实现 | 当前没有服务控制 command 或 service 层 |
| 检测服务端口 | 未实现 | 当前没有端口检测实现 |
| 检测进程状态 | 未实现 | 当前没有进程检测实现 |
| 读取服务日志 | 未实现 | 当前没有日志读取 command |
| 服务异常提示 | 未实现 | 当前没有服务状态模型或 UI |
| 服务白名单机制 | 未实现 | 当前没有服务配置或 allowlist |

### 5.4 脚本执行能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 执行预置脚本 | 未实现 | 当前没有脚本执行 command |
| 查询脚本执行状态 | 未实现 | 当前没有任务模型 |
| 获取脚本执行日志 | 未实现 | 当前没有 stdout / stderr 采集封装 |
| 取消执行任务 | 未实现 | 当前没有任务生命周期管理 |
| 记录执行参数和结果 | 未实现 | 当前没有本地持久化或日志服务 |
| 脚本白名单和参数校验 | 未实现 | 当前没有安全校验模块 |

### 5.5 本地数据能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 本地配置存储 | 未实现 | 当前没有配置服务 |
| 用户偏好设置 | 未实现 | 当前没有偏好设置模块 |
| 服务状态缓存 | 未实现 | 当前没有服务状态模型 |
| 操作记录缓存 | 未实现 | 当前没有缓存或存储层 |
| 本地数据库存储 | 未实现 | 当前没有 SQLite 或数据库模块 |
| 配置导入导出 | 未实现 | 当前没有导入导出 API |

### 5.6 日志、诊断与安全能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 前端错误日志 | 未实现 | 当前没有前端日志采集 |
| 原生层运行日志 | 未实现 | 当前没有 Rust 日志服务 |
| 脚本执行日志 | 未实现 | 当前没有脚本任务实现 |
| 服务控制日志 | 未实现 | 当前没有服务控制实现 |
| 接口请求日志 | 未实现 | 当前没有请求层 |
| 本地诊断信息导出 | 未实现 | 当前没有诊断导出 command |
| 日志滚动 | 未实现 | 当前没有日志文件管理 |
| Tauri command 最小暴露 | 部分实现 | 当前仅暴露 `greet` 示例 command；业务 command 尚未设计 |
| 参数校验 | 部分实现 | `greet` 只接收 `name: &str`，尚未形成业务校验规范 |
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
