## 目录结构

```text
client-ui/       Vue 前端工程，提供两套 Tauri 壳共用的 UI 产物
client-tauri2/   Tauri 2 桌面壳，用于 Windows 和新版 Linux
client-tauri1/   Tauri 1 legacy 桌面壳，用于 libwebkit2gtk-4.0 国产 Linux 环境
```

常用构建命令：

```bash
pnpm build                 # 仅构建共用前端
pnpm build:win             # Tauri 2 Windows: msi, nsis
pnpm build:mac             # Tauri 2 macOS: app, dmg
pnpm build:linux           # Tauri 2 Linux: deb, rpm
pnpm build:linux:legacy    # Tauri 1 Linux: deb, rpm
```

## 发布矩阵

| 平台 | Tauri 版本 | 软件名 | 架构 | 安装包 |
| --- | --- | --- | --- | --- |
| Windows | v2 | cpmsClient | x86_64, arm64 | msi, nsis |
| macOS | v2 | cpmsClient | x86_64, arm64 | app, dmg |
| Linux | v1 | cpmsClient-v1 | x86_64, arm64 | deb, rpm |
| Linux | v2 | cpmsClient-v2 | x86_64, arm64 | deb, rpm |

GitHub Actions 发布配置位于 `.github/workflows/build-release.yml`。打 tag `v*` 时会构建全部矩阵并上传 Release 附件；普通 push 和 pull request 只执行构建校验。

## 架构设计
前端路由：
- /  空layout页面   只有一个iframe作为容器
- /example  示例页面   提供客户端能力检测状态
桌面通知：
- 根据子窗口 创建桌面通知窗口  默认不显示  只有监听到事件消息时才显示  并渲染通知内容   可以通过关闭按钮隐藏窗口
托盘：
- 客户端启动后  创建托盘图标  可以通过托盘图标显示/隐藏客户端窗口
自启动：
- 客户端首次启动后  默认自启动  可以通过托盘图标设置自启动
请求：
- 满足视图端请求客户端
- 满足客户端请求线上服务端
- 满足客户端链接socket
通信：
- 满足视图端向客户端发送事件
- 满足客户端向视图端发送事件

## 项目需求

### 需求 1：渲染线上 iframe 容器 地址

客户端启动后请求线上服务，获取 iframe 容器地址，视图端根据返回地址渲染业务页面。

流程：

```text
启动客户端
  -> 客户端请求线上服务
  -> 获取 iframe URL
  -> 视图端从客户端获取 iframe URL 并根据 iframe URL 渲染业务页面
```

### 需求 2：连接本地 socket 服务

客户端启动后连接本地 socket 服务，等待任务推送，并将收到的任务二次转发（待描述具体实现）。

流程：

```text
启动客户端
  -> 连接本地 socket 服务
  -> 等待任务推送
  -> 解析任务消息
  -> 二次转发任务消息  （获取iframe实例内的token  携带给要转发的任务）
```

