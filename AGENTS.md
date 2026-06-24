# CPMS Client Agent 说明

本文档面向 `client` 子项目。它是桌面客户端宿主，负责 Tauri 桌面壳、宿主 UI、iframe 容器和原生能力；完整业务页面与业务流程在同级项目 `hub-web` 中维护。

## 文档分工

| 文档 | 用途 |
| --- | --- |
| `PRODUCT.md` | 桌面客户端产品、业务流程和接口契约。 |
| `DESIGN.md` | 桌面客户端视觉系统、设计令牌和组件规范。 |
| `AGENTS.md` | 编码代理协作说明；只记录结构、边界、命令和高风险约定。 |

## 项目定位

`client` 是一个 pnpm workspace，包含一套宿主 UI 和两个 Tauri 壳：

| 工程 | 角色 |
| --- | --- |
| `client-ui` | Vue 3 + TypeScript 宿主 UI，提供入口页、iframe 容器、通知窗口、调试面板和 Tauri 桥接封装。 |
| `client-tauri1` | Tauri 1 legacy 壳，面向麒麟、统信等国产 Linux；当前唯一维护的客户端原生壳。 |
| `client-tauri2` | Tauri 2 桌面壳，历史版本保留；不再更新。 |

边界原则：

- 业务页面不在 `client-ui` 中重做，由 `hub-web` 以 iframe 形式提供。
- 当 iframe 指向 `hub-web` 时，客户端服务端也应指向 `hub-server`，与 `hub-web` 使用同一后端；`hub-server` 仅作只读参考。
- `client` 负责桌面能力：托盘、自启动、窗口控制、通知、本地 socket、打印任务转发、日志、网络检测、签名请求、会话 token、iframe 桥接。
- 客户端原生能力只修改 `client-tauri1`；不要同步或补改 `client-tauri2`。
- 修改跨业务契约时，同时检查 `hub-web` 的桥接/API 调用；`hub-server` 只读参考其服务端接口。

## 目录结构

```text
client/
  package.json              # workspace 命令入口
  pnpm-workspace.yaml
  PRODUCT.md
  DESIGN.md

  client-ui/
    src/
      main.ts               # Vue 启动入口
      App.vue               # 按窗口 label 分发主窗口/通知窗口
      views/                # home、entry、notification、debug/log 等宿主视图
      components/           # WindowHeaderBar 等宿主组件
      composables/          # iframe、事件、通知、日志桥接
      stores/               # runtime、task、log、network、user 等宿主状态
      api/tauri/            # Tauri invoke/event 类型化封装
      utils/hubBridge.ts    # 注入 iframe 的 __HUB_CLIENT__ 桥

  client-tauri2/
    src-tauri/src/
      lib.rs                # app shell、命令注册、托盘、自启动、panic hook
      window.rs             # 窗口控制
      event_bridge.rs       # client-ui ↔ Tauri 事件桥
      iframe.rs             # iframe 地址、状态与 payload 查询
      socket.rs             # 本地 socket 监听与任务转发
      printclient.rs        # PrintClient 发现与配置读取
      token_refresh.rs      # token 失效重取与重试
      services/             # HTTP、签名、日志、会话、打印、网络等服务

  client-tauri1/
    src-tauri/              # legacy Tauri 1 壳；结构与 tauri2 相近，API 适配不同
```

## 关键契约

- **iframe 地址**：用户在入口页配置 `hub-web` 地址；宿主保存并渲染 iframe。
- **服务端地址**：当 iframe 为 `hub-web` 时，客户端服务端为 `hub-server`；签名请求、打印转发、serverAddress/deviceId 等后端契约按只读的 `hub-server` 参考。
- **通信信封**：Tauri 事件桥使用 `{ id, type, payload, time }` 形式，新增事件时保持同一约定。
- **iframe payload**：`hub-web` 通过 `postMessage` 推送或响应 `cpms:token`、`cpms:platform`、`cpms:serverAddress`、`cpms:deviceId`、`cpms:refresh`。
- **token 模型**：token 只保存在 Tauri 会话内存中，不作为长期偏好落盘；鉴权失败时经 iframe 查询并重试。
- **platform 契约**：iframe 推送的 platform 同时影响 CPMS 请求头和打印参数。
- **打印链路**：本地 socket 收到任务后，由 Tauri 携带会话 token 转发到 CPMS；客户端不重新实现完整业务列表页面。
- **壳维护策略**：当前只维护 `client-tauri1`；`client-tauri2` 作为历史版本保留，不做同步更新。

## 常用命令

在 `client` 目录执行：

```sh
pnpm install
pnpm dev                  # client-ui 开发服务
pnpm build                # 构建宿主 UI
pnpm lint
pnpm fmt
pnpm tauri:v1 dev
pnpm build:linux:legacy
```

Rust 校验在当前维护壳目录执行：

```sh
cd client-tauri1/src-tauri
cargo check
```

## UI 约定

- 视觉系统以 `client-ui/src/assets/styles/tokens.css` 的 `--cpms-*` 令牌为准。
- 主窗口、通知窗口和调试抽屉复用 `WindowHeaderBar.vue` 的窗口语义。
- 宿主 UI 保持工具型、紧凑、可扫描；不要把业务页面搬进宿主。
- 新增图标统一使用 `@lucide/vue` 风格；不再新增旧图标包、旧自动图标类或旧原子样式图标类。

## 验证建议

- UI、TypeScript、桥接封装变更：运行 `pnpm build`，必要时运行 `pnpm lint`。
- Tauri v1 变更：运行 `client-tauri1/src-tauri` 下的 `cargo check`，并注意国产 Linux 真机差异。
- Tauri v2：历史版本保留，不作为日常开发、验证或同步目标，除非用户明确单独要求。
- 跨 iframe / token / 打印协议变更：同时检查 `hub-web` 发送端/接收端，并只读参考 `hub-server` 接口。

## 协作规则

- 修改前先读取相关模块，不凭文件名猜行为。
- 不改 `node_modules`、构建产物、日志、截图和无关 lockfile。
- 不回滚用户已有未提交变更。
- 文档保持当前事实和高价值约定，避免记录历史流水账、已删除代码清单或过细实现说明。
