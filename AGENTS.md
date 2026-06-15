# CPMS Client Agent 说明

设计需求源文档：[DESIGN.md](DESIGN.md)。本文档列出系统能力 / 系统需求 / 业务需求三类清单及其实现状态，并描述当前真实的项目结构与工作约定。

## 1. 项目定位

CPMS Client 是一个 pnpm workspace，包含一套共用的 Vue 3 视图端和两个 Tauri 桌面壳：

| 工程 | 角色 |
| --- | --- |
| `client-ui` | Vue 3 + TypeScript + Vite(+) 视图端，两套壳共用同一份 UI 产物 |
| `client-tauri2` | Tauri 2 壳：Windows / macOS / 新版 Linux（webkit2gtk-4.1） |
| `client-tauri1` | Tauri 1 legacy 壳：麒麟 V10 / 统信等国产 Linux（webkit2gtk-4.0、GTK 3.18），带 vendored tao/wry 补丁 |

架构基调（DESIGN.md）：以 Tauri 基座承载客户端专属能力（桌面通知、托盘、自启动、网络检测、服务请求、事件通信）；**主窗体为无头窗口**，headerbar 由视图端绘制；业务页面由 iframe 内的线上 hub-platform 提供，客户端不做登录、不镜像业务状态。

## 2. 系统能力列表（客户端基座能力）

| # | 能力 | 状态 | 实现位置 |
| --- | --- | --- | --- |
| C1 | 主窗体：无头窗口 + 视图端 headerbar（logo+标题+固定/收起/全屏/关闭） | 已实现 | 两壳 `tauri.conf.json`（`decorations:false`，label `main`）；`client-ui/src/components/layout/WindowHeaderBar.vue`；`client-ui/src/views/home/index.vue` |
| C2 | 桌面通知子窗口：400×400、屏幕右下角、headerbar+通知内容、单实例、关闭即隐藏 | 已实现 | `client-ui/src/api/tauri/notification.ts`（启动预创建隐藏窗口 `prepareNotificationWindow`、定位、推送）；`client-ui/src/views/notification/index.vue`（渲染）。符合 DESIGN「默认加载但不显示，事件到来才显示」 |
| C3 | 系统托盘：左键显隐主窗口；右键菜单 显示/隐藏/开关自启动/退出 | 已实现 | tauri2 `lib.rs setup_tray`；tauri1 `lib.rs build_tray/handle_tray_event` |
| C4 | 开机自启动：首次启动默认开启，托盘与命令可切换 | 已实现 | `lib.rs init_autostart_on_first_launch`、`autostart_*` 命令（v1 用 auto-launch，v2 用 tauri-plugin-autostart） |
| C5 | 网络检测：在线/离线监听，状态变化推送视图端 | 已实现 | 两壳 `hub/network_service.rs`（探测 8.8.8.8:53，变化 emit `cpms:hub-network-changed`） |
| C6 | 服务请求：CPMS 签名请求（access_sign + client/platform 公共头 + Authorization） | 已实现 | `hub/http_service.rs`、`hub/crypto_service.rs`（access_sign：AES-128-ECB + MD5） |
| C7 | 客户端↔视图端事件通信（交互指令/客户端方法/请求代理） | 已实现 | `lib.rs setup_client_event_bridge/handle_view_event`；`client-ui/src/api/tauri/events.ts` |
| C8 | 本地 socket 接入：PrintClient 检测、端口发现、ws 连接、任务监听与转发 | 已实现 | `lib.rs discover_print_client_socket_url / start_local_socket_worker` |
| C9 | iframe 桥：`__HUB_CLIENT__` 注入 + payload（token）查询回传 | 已实现 | `client-ui/src/utils/hubBridge.ts`、`composables/useIframePayloadBridge.ts`、`lib.rs client_*_iframe_payload` |
| C10 | 国产 Linux legacy 适配（glibc 2.23 / GTK 3.18 / webkit 2.20 / 静态 OpenSSL） | 已实现 | `client-tauri1`（vendored tao/wry 补丁 + CI 符号硬门禁，详见 `.github/CI-TROUBLESHOOTING.md`） |
| C11 | 调试抽屉：客户端能力状态 / 调试客户端能力 / 日志查看 | 已实现 | 抽屉双页签：能力状态与调试在 `client-ui/src/views/example/index.vue`；独立日志面板在 `views/logs/index.vue`（`stores/log.ts` 缓冲 500 条，`useClientLogBridge` 汇集客户端日志事件、客户端事件流与错误队列，支持复制/清空，展示日志文件路径） |
| C12 | 客户端日志系统：文件落盘 + 接收前端推送 + 启动关键链路埋点 | 已实现 | 两壳 `services/log_service.rs`（写 `app_log_dir/cpms-client.log`，5MB 单档轮转，UTC 时间戳，未引入 chrono 以保护 legacy 依赖锁；客户端日志同步 emit `cpms:client-log` 给视图端）；命令 `push_client_log`（前端/iframe 推送，仅落盘不回发）与 `get_client_log_state`；埋点覆盖启动各阶段、自启动、iframe 获取/回退、socket 解析/连接/断开/任务转发、token 失效重取、登录态与系统能力生命周期、主窗口关闭隐藏 |
| C13 | 单实例运行 | 已实现 | 两壳 `single_instance.rs`（回环端口 `127.0.0.1:51987` 作进程锁 + magic 握手，零插件依赖以兼顾 legacy 依赖链）；二次启动握手确认本应用后唤醒首实例显示窗口并退出，端口被异端占用则放行不保护 |
| C14 | 转发失败持久化重试 | 已实现 | 两壳 `socket.rs`：转发失败（含 token 重取后仍失败）落盘到 `app_data_dir/pending-forwards/`，重试 worker 每 30s 重发，成功出队、超 10 次丢弃并记日志（弥补移除缓存 worker 后单一路径的可靠性） |
| C15 | TLS 证书校验可配置 | 已实现 | 默认校验证书；仅 env `CPMS_ALLOW_INSECURE_TLS=1` 时放开（`services/http_service.rs::allow_insecure_tls`，覆盖代理/上传/CPMS 请求三处） |
| C16 | token 落盘加密 | 已实现 | `services/token_store.rs`：每安装随机本地密钥（受限权限文件）AES 加密 token 字段，`preferences` 读写透明加解密，旧明文兼容、解密失败 fail-open。**限制**：本地密钥静态加密（防误读/误同步），强度不及 OS keychain |
| C17 | Rust panic 入日志 | 已实现 | 两壳 run() 装 `panic::set_hook`，把 Rust panic 写入 `cpms-client.log` |
| C18 | 主窗口几何持久化 | 部分实现 | tauri2 `window.rs`（关闭到托盘时存大小/位置，启动恢复，best-effort）；tauri1 暂跳过（无头固定窗 + webkit2.20 不可测，价值低，见第 8 节） |

## 3. 系统需求列表（入口事件 / 常驻事件 / 通信 / 请求）

| # | 需求 | 状态 | 说明与位置 |
| --- | --- | --- | --- |
| S1 | 入口：客户端启动后请求服务端获取 iframe 容器地址并缓存 | 已实现 | `lib.rs refresh_iframe_container`（`/api/client/iframe-config`，host 白名单校验，失败回退默认地址，状态缓存于 `AppRuntimeState`） |
| S2 | 入口：视图端向客户端取 iframe 地址并渲染业务页面 | 已实现 | `client_get_iframe_container_state` / `client_refresh_iframe_container` 命令 + `cpms:client-iframe` 事件；`views/home/index.vue` 渲染 |
| S3 | 常驻：检测 cpms 客户端（PrintClient）是否存在并读取配置取 websocket 端口 | 已实现 | `lib.rs print_client_candidate_dirs / socket_url_from_config_file`（解析 `DriverClient.ini` / `config.conf` / `config.ini`，支持 env 覆盖） |
| S4 | 常驻：连接本地 websocket 服务并监听任务推送 | 已实现 | `lib.rs start_local_socket_worker`（断线 3s 重连） |
| S5 | 常驻：解析任务消息拿到文件路径，携带 token 转发任务 | 已实现 | `lib.rs is_print_task_message` → `hub/print_service.rs forward_socket_task_message`（multipart 上传） |
| S6 | 通信：视图端→客户端 固定/收起/全屏/关闭窗口事件 | 已实现 | `lib.rs handle_view_event`（`client.window.pin/unpin/minimize/fullscreen/exit-fullscreen/close`）；headerbar 按钮触发 |
| S7 | 通信：视图端→客户端 作业列表/打印机列表/选择打印机/更新 token 事件 | 已实现 | `handle_view_event`（`client.jobs.list`、`client.devices.list`、`client.device.select`、`client.auth.update-token`），结果以 `*.result` 事件回推 |
| S8 | 通信：客户端→视图端 查询 token 事件 | 已实现 | `client.iframe_payload.request` → 视图端 postMessage 查询 iframe → `client_submit_iframe_payload` 回传 |
| S9 | 请求：视图端请求客户端（iframe 地址/作业列表/打印机列表/选择打印机） | 已实现 | 同名命令 + 事件双通道；`client-ui/src/api/tauri/` 为类型化命令层 |
| S10 | 请求：客户端请求服务端（iframe 地址/作业列表/打印机列表/转发任务） | 已实现 | `hub/commands.rs`（`get_job_list`、`get_available_devices`、`select_direct_device`）、`hub/print_service.rs`（转发） |
| S11 | 请求代理：视图端经客户端代理 HTTP 请求 | 已实现 | `client_http_request` → `hub/http_service.rs execute_client_http_request` |

## 4. 业务需求列表（DESIGN.md 项目需求）

| # | 需求 | 状态 | 说明 |
| --- | --- | --- | --- |
| B1 | 需求 1：渲染线上 iframe 容器地址（启动→请求→缓存→视图端渲染） | 已实现 | 见 S1/S2；加载中有 loading 态，失败回退默认地址并提示原因 |
| B2 | 需求 2：连接本地 socket 服务，等待任务推送并二次转发 | 已实现 | 见 S3–S5；转发结果以 `client.socket_task.forwarded / forward_failed` 事件回推视图端 |
| B3 | 需求 3：Token 机制——登录后视图端推送 token | 已实现 | `save_auth_token` 命令 + `client.auth.update-token` 事件，持久化于 `hub-preferences.json` |
| B4 | 需求 3：Token 机制——客户端主动从 iframe 实例获取 token | 已实现 | C9 的 payload 查询链路（启动 2s 后及按需触发） |
| B5 | 需求 3：Token 机制——请求失败清理缓存 token，重新获取，不一致则重试 | 已实现 | 两壳 `token_refresh.rs::with_token_retry` 通用包装：鉴权失败（401/403）→ 清缓存 token → 向 iframe 重查（10s 超时）→ token 不一致则重发一次。**覆盖全部「客户端→服务端」通信**：socket 转发、作业列表、设备列表、选择机器 |
| B6 | 接口：普通作业列表 `POST /cpms/api/jobs/list` | 已实现 | `hub/commands.rs get_job_list`，公共头齐全 |
| B7 | 接口：设备列表 `GET /cpms/api/userManager/listAvailDevices` | 已实现 | `get_available_devices` |
| B8 | 接口：选择机器 `POST /cpms/api/userManager/updateDirectDeviceId` + 本地持久化 | 已实现 | `select_direct_device`（服务端更新 + preferences 持久化，返回约定的本地响应结构） |
| B9 | 接口：转发任务 `POST /cpms/api/jobs/xps/exec`（multipart，含 printProperties.* Query） | 已实现 | `hub/print_service.rs upload_print_payload`；旧流程 `uploadJobByWebOrH5` 未启用（按当前主流程实现） |
| B10 | socket 推送任务响应体解析（filePath + printProperties） | 已实现 | `parse_socket_task_payload` 兼容双层 JSON 字符串包装 |

## 5. 项目目录结构（真实现状）

```text
client/
  AGENTS.md / DESIGN.md / README.md
  package.json                  # workspace 脚本入口（dev/build/各平台打包）
  pnpm-workspace.yaml

  client-ui/                    # 视图端（两壳共用）
    index.html
    uno.config.ts / vite.config.ts
    public/                     # 静态资源（tauri.svg 等）
    src/
      main.ts                   # 挂载入口；注入 __HUB_CLIENT__ 桥
      App.vue                   # 按窗口 label 分发：main → home，notification → notification
      assets/styles/tokens.css  # 设计令牌：全窗口共用的颜色/字号/圆角/间距变量
      views/
        home/index.vue          # 主窗口：headerbar + iframe 容器 + 调试抽屉（能力检测/客户端日志双页签）
        notification/index.vue  # 通知子窗口：headerbar(标题+关闭) + 通知内容
        example/index.vue       # 调试抽屉：能力状态与各项检测
        logs/index.vue          # 调试抽屉：客户端日志面板（复制/清空/等级着色）
      components/
        layout/WindowHeaderBar.vue  # 共用窗口 headerbar（拖拽区 + 固定/收起/全屏/关闭）
        common/ErrorNotice.vue
      composables/              # 事件桥/通知桥/日志桥/iframe 容器与 payload 桥/错误与通知队列
      stores/                   # app(配置/错误/通知)、runtime(iframe 态)、task(socket 任务)、log(日志缓冲)、user(本地 token 展示)、network
      api/
        tauri/                  # 命令层：client(invoke 封装)、desktop、events、log、notification(通知子窗口 400×400)
        config.ts               # 客户端配置/token 读取（localStorage）
      types/                    # app/common/task 分类类型
      utils/hubBridge.ts        # 注入 iframe 的 __HUB_CLIENT__（统一走 api/tauri/client.invokeCommand）

  client-tauri2/                # Tauri 2 壳
    package.json / scripts/build.js
    src-tauri/
      tauri.conf.json           # main 窗口 800×600、decorations:false；bundle 目标
      capabilities/default.json # main + notification 窗口权限
      src/
        lib.rs                  # app shell：builder、托盘、自启动、panic 钩子、run()、共享 consts/结构体
        single_instance.rs      # 单实例保护（端口锁 + 握手）
        window.rs               # 主窗口控制命令 + 复用辅助函数 + 几何持久化(tauri2)
        event_bridge.rs         # 视图端↔客户端事件桥（窗口/作业/设备/token 指令分发）
        iframe.rs               # iframe 地址获取/校验/回退、状态缓存、payload(token) 查询
        printclient.rs          # 本地 PrintClient 发现（解析配置取 websocket 端口）
        socket.rs               # 本地 socket worker + 任务转发 + 失败重试队列
        token_refresh.rs        # token 失效重取通用包装（转发与 CPMS 请求共用，需求3）
        result.rs               # CommandResult<T> 统一返回结构
        services/               # 业务服务层（见下）

  client-tauri1/                # Tauri 1 legacy 壳（国产 Linux）
    package.json / vite.config.ts
    src-shims/tauri/            # 给 client-ui 模拟 @tauri-apps/api v2 接口的别名 shim
    src-tauri/
      tauri.conf.json           # 同上（v1 schema）+ allowlist + systemTray
      tauri.linux(.legacy).conf.json  # productName cpmsClient-v1 等覆盖
      vendor/tao-0.16.11/ wry-0.24.12/  # 麒麟 GTK3.18/webkit2.20 真机补丁，勿动
      src/                      # 与 tauri2 同构（lib.rs/window/event_bridge 内 v1 API 差异）

  # services/ 模块（两壳内容一致，仅 v1/v2 API 适配差异）：
  #   commands.rs        tauri command（启动态/认证/作业/设备/系统能力/日志/工具）
  #   http_service.rs    CPMS URL 拼接、签名请求头、HTTP 代理
  #   log_service.rs     客户端日志：文件落盘(app_log_dir/cpms-client.log) + cpms:client-log 事件
  #   token_store.rs     token 落盘 AES 加密（本地随机密钥）
  #   crypto_service.rs  access_sign 签名（AES-128-ECB + MD5）
  #   print_service.rs   socket 推送的打印任务转发上传（/cpms/api/jobs/xps/exec）
  #   network_service.rs 网络在线监测（变化 emit cpms:hub-network-changed）
  #   preferences.rs     hub-preferences.json 持久化（token/服务器/设备等）
  #   events.rs          cpms:hub-system-state / network-changed 事件 emit
  #   models.rs          领域模型
  #   mod.rs             导出 + token 缓存辅助（cached/clear/save_cached_auth_token）
```

## 6. 窗口与样式约定

- 所有窗口内容统一引用 `client-ui/src/assets/styles/tokens.css` 中的 `--cpms-*` 设计令牌（颜色、字号、圆角、间距、headerbar 高度、danger 色），组件内禁止写裸色值/裸字号。
- `tokens.css` 用 `:root:root` 把 Element Plus 关键变量（`--el-text-color-*`/`--el-border-color*`/`--el-fill-color-*`/`--el-border-radius-base`/`--el-font-size-base`）对齐到 `--cpms-*`，让 el-button/el-input/el-tag/el-drawer/el-tabs/el-alert 与自绘外壳同一套视觉语言；`--el-color-primary`（强调蓝）保留 EP 默认。
- 主窗口、通知子窗口、**调试抽屉**外壳统一使用 `WindowHeaderBar.vue`：主窗口 logo+标题+固定/收起/全屏/关闭，通知窗口/抽屉 标题+关闭（抽屉用 el-drawer `#header` 插槽嵌入）；headerbar 即拖拽区（`data-tauri-drag-region`）。
- 主窗口关闭按钮与系统关闭请求一致：隐藏到托盘而非退出；退出从托盘菜单走 `system_destroy` 后 `exit(0)`。
- 通知子窗口固定 400×400、右下角、置顶、不进任务栏，同一时刻只显示一条通知。

## 7. 开发与构建命令

workspace 根目录：

```bash
pnpm dev                  # client-ui 开发服务（端口 1420）
pnpm build                # 仅构建共用前端（vue-tsc --noEmit && vp build）
pnpm lint / pnpm fmt      # vp lint / vp fmt

pnpm tauri:v2 dev         # Tauri 2 壳开发
pnpm tauri:v1 dev         # Tauri 1 壳开发

pnpm build:win            # Tauri 2 Windows: msi + nsis
pnpm build:mac            # Tauri 2 macOS: app + dmg
pnpm build:linux          # Tauri 2 Linux: deb + rpm
pnpm build:linux:legacy   # Tauri 1 Linux: deb + rpm（国产环境）
```

Rust 校验：在 `client-tauri{1,2}/src-tauri` 下 `cargo check`。CI（`.github/workflows/build-release.yml`）按环境分组构建，legacy 链路含 glibc/GTK 符号硬门禁；排错见 `.github/CI-TROUBLESHOOTING.md`。

## 8. 已知债务与注意事项

- **services/ 双壳复制**：`client-tauri1` 与 `client-tauri2` 的 `src/services/` 内容一致（仅 `path_resolver/emit_all/open` 等 v1 API 差异）；`lib.rs`/`window.rs`/`event_bridge.rs` 也按 v1/v2 分别维护（`get_window` vs `get_webview_window`、`listen_global` vs `listen_any`、SystemTray vs TrayIconBuilder、auto_launch vs autostart 插件、tauri2 独有窗口几何持久化）。`iframe.rs`/`socket.rs` 仅 import 行差一处（v1 走 `Manager` 无 `Emitter`），复制后需改回 `use tauri::{AppHandle, Manager};`；`printclient.rs`/`result.rs`/`single_instance.rs`/`token_refresh.rs`/`token_store.rs` 两壳逐字一致，可直接复制。改动必须双侧同步。后续可抽共享 crate，但受 tauri1 vendored 补丁与 CI 链路约束，暂未拆分。
- **环境变量**：`CPMS_BASE_URL`/`CPMS_IFRAME_CONFIG_PATH`/`CPMS_IFRAME_ALLOW_HOSTS`（iframe 地址与白名单）、`CPMS_PRINTCLIENT_*`（PrintClient socket 发现）、`CPMS_ALLOW_INSECURE_TLS=1`（放开 TLS 校验，默认校验）、`WEBKIT_DISABLE_COMPOSITING_MODE`（legacy 白屏规避，默认开）。
- **tauri1 真机回归**：无头窗口、headerbar 拖拽区、400×400 通知窗口需在麒麟/统信真机复测（webkit 2.20 渲染行为与新版不同）。
- **打印能力范围**：客户端只做「socket 推送任务 → 携带 token 转发到 xps/exec」这一条链路（DESIGN 需求 2）。已移除 USB 直连打印、系统级虚拟打印机相关命令（add/disable/fix_printer、init_usb_printer/get_usb_state）、Windows 端打印缓存扫描 worker、本地 TCP 文件接收服务（socket_server，start/stop_socket_server）。如需重新引入须同时补回 Rust 服务、命令注册、UI 封装与本说明。
- **api/tauri 命令层**：现存 `client`(invoke 封装)、`desktop`、`events`、`log`、`notification`(通知子窗口)，均被视图端实际使用；iframe 侧统一经 `utils/hubBridge.ts`（透传 CommandResult 原始结构，不要改为 unwrap）。
- **已清理的脚手架/死代码（2026-06-15）**：删除模板示例 `greet` 命令、`vue.svg`/`vite.svg`、未用的 `panel-title` uno 快捷类、根目录 `job-logs.txt`、死代码类型目录 `types/hub/`；删除未被任何 view 引用的浏览器侧 HTTP 客户端（`api/{http,cpms,localService}/client.ts`）与类型化命令镜像（`api/tauri/{hub-client,version,hub-crypto,external}.ts`）；删除 UI 完全未用的命令 `window_maximize`/`window_unmaximize`、`ping_server`、SM4 整条链路（`sm4_encrypt` 命令 + `crypto_service::sm4_encrypt_hex` + `sm4` 依赖）。保留但供 iframe 备用的工具命令：`get_app_version`、`open_external`、`close_window_with_confirm`、`sign_request`。
- **结构整理（2026-06-15）**：Rust `hub/`→`services/`、根 `models.rs`→`result.rs`；UI `api/desktop/notification.ts`→`api/tauri/notification.ts`、`api/request/config.ts`→`api/config.ts`、`types/task/todoTask.ts`→`todo-task.ts`；`lib.rs`(1200+ 行) 拆为 `window/event_bridge/iframe/printclient/socket` 模块，lib.rs 仅留 app shell。

## 8.1 待补充能力（需外部资产或需真机验证，暂未启用）

| 能力 | 状态 | 需要你提供 / 注意 |
| --- | --- | --- |
| 自动更新（updater） | 未启用 | 需「更新服务器/静态托管的 latest.json 地址」+「`tauri signer generate` 生成的签名密钥对」（公钥进 `tauri.conf.json`，私钥签发布物、严禁入库）。建议仅 Windows/macOS 启用；国产 deb 走仓库源。提供后接 `tauri-plugin-updater` 即可。 |
| 代码签名 / 公证 | 未启用 | Windows 代码签名证书、macOS Developer ID + notarize 凭据，属 CI/发布机密。提供后在 bundle 配置接入。 |
| Webview CSP | 未启用（`csp: null`） | 收紧 CSP 需精确知道 iframe 内 hub-platform 的静态资源源，且 webkit2.20(legacy) 不可在本机验证、配错即白屏。建议值（待你按真实资源域确认后启用）：`default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self' https: http:; frame-src https: http:`。 |
| token OS keychain | 用 C16 本地加密代替 | 如需更强凭据保护，可换 Windows DPAPI / Linux Secret Service；但麒麟/统信 legacy 上 secret-service 未必可用，需真机确认。 |

## 9. Agent 工作约定

- 输出使用 UTF-8；中文注释。
- 修改代码前先读取当前实现；hub/ 与 lib.rs 的改动同步到两个壳。
- 文档只描述当前项目已经存在的结构、配置和命令。
- 默认遵循 Vite+ 工作流，优先使用 `vp` 命令。
- 不回滚用户已有变更；只做和当前任务相关的最小必要修改。
- 视觉改动必须走 `tokens.css` 设计令牌，保持窗口风格统一。
