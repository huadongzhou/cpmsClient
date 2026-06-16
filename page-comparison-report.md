# 业务页面对比报告：hm-client vs hub-platform

> 对比范围：原端 `hm-client/entry/src/main/ets` 与复刻端 `hub-platform/src`
> 目的：核对 `hub-platform` 是否完整、规范复刻 `hm-client` 的业务页面

---

## 一、页面对照总表

| 序号 | 原端页面/视图 | 复刻端页面 | 对应关系 | 对比状态 |
|------|--------------|-----------|---------|---------|
| 1 | `pages/PolicyPage.ets` | `pages/PolicyPage.vue` | 1:1 | ✅ 已对比 |
| 2 | `pages/LoginPage.ets` | `pages/LoginPage.vue` | 1:1 | ✅ 已对比 |
| 3 | `pages/MainPage.ets` | `pages/MainLayout.vue` | 功能对应 | ✅ 已对比 |
| 4 | `view/HomePage.ets` | `pages/HomePage.vue` | 1:1 | ✅ 已对比 |
| 5 | `view/MinePage.ets` | `pages/MinePage.vue` | 1:1 | ✅ 已对比 |
| 6 | `view/PrintDeviceListRefreshLoadPageView.ets` | `pages/PrintDevicePage.vue` | 1:1 | ✅ 已对比 |
| 7 | `view/JobLisTabsPageForC.ets` + `JobListPageForC.ets` + `PrintJobListRefreshLoadPageViewForC.ets` + `JobListRefreshLoadPageViewCopy.ets` + `JobListRefreshLoadPageViewScan.ets` | `pages/PrintJobPageForC.vue` | 多合一 | ✅ 已对比 |
| 8 | `view/UserMessagePage.ets` + `view/UserMessageListRefreshLoadPage.ets` | `pages/JobMessagePage.vue` | 多合一 | ✅ 已对比 |
| 9 | `biz/components/CustomTitleBar.ets` | `components/layout/CustomTitleBar.vue` | 1:1 | ✅ 已对比 |
| 10 | `biz/components/CustomBottomBar.ets` | `components/layout/CustomBottomBar.vue` | 1:1 | ✅ 已对比 |
| 11 | `view/HomeCurrentPrinterModule.ets` | 内联在 `pages/HomePage.vue` | 合并 | ✅ 已对比 |
| 12 | `view/WebDialogPage.ets` + `WebDialogPageAbountUs.ets` + `WebDialogPageManual.ets` | `components/common/WebDialog.vue` | 多合一 | ✅ 已对比 |
| 13 | `view/PasswordDialogPage.ets` | 内联在 `pages/MinePage.vue` | 合并 | ✅ 已对比 |
| 14 | `view/FailureLayout.ets` + `LoadingLayout.ets` + `LoadMoreLayout.ets` + `NoMoreLayout.ets` + `RefreshLayout.ets` | `components/common/ListState.vue` | 多合一 | ✅ 已对比 |
| 15 | `view/JobLisTabsPage.ets` + `PrintJobListRefreshLoadPageView.ets` + `JobCardItem.ets` | `pages/PrintJobPage.vue` + `components/common/JobCardItem.vue` | 新增 | ✅ 已补充 |

---

## 二、逐页详细对比

### 1. 隐私政策页（PolicyPage）

| 对比项 | 原端 `hm-client` | 复刻端 `hub-platform` | 结论 |
|--------|-----------------|----------------------|------|
| 路由 | 页面路由跳转 | `/policy` | 对齐 |
| 布局 | `GridRow/GridCol` 响应式栅格，移动端优先 | 居中卡片，无栅格断点 | 复刻版缺少响应式栅格 |
| 不同意 | `killAllProcesses()` 杀进程 | `window.close()` 关闭窗口 | 行为不一致 |
| 同意 | 写 Preference + 跳转登录 | `savePolicyAgreed()` + Pinia + `router.replace("/login")` | 逻辑对齐 |
| 弹窗 | `CustomDialogController` + `WebDialogPolicy` | `<WebDialog>` | 组件形态不同，能力对齐 |
| 图片 | `startIcon.png` | `startIcon.png` ✅ | 已复制 |
| 死代码 | `PolicyBottom` 组件被注释但残留 | 较精简 | 原版需清理 |

**主要问题**：复刻版缺少响应式栅格；原版存在大量未清理死代码。

---

### 2. 登录页（LoginPage）

| 对比项 | 原端 `hm-client` | 复刻端 `hub-platform` | 结论 |
|--------|-----------------|----------------------|------|
| 布局 | 左右分栏 + 装饰圆 + 响应式 `sm/md/lg` | 左右分栏卡片 + CSS media query | 视觉基本对齐，缺少装饰圆 |
| 自动登录 | ✅ 启动时尝试自动登录 | ✅ 挂载时检测已认证态并跳转；存在历史服务器时自动加载认证方式 | 已对齐 |
| 服务器检测 | ✅ 输入框防抖 3s 自动检测 | ⚠️ 必须点击“检测服务端”按钮 | 交互流程不同 |
| 本地账号密码 | ✅ | ✅ | 对齐 |
| AD 域登录 | ✅ | ✅ | 对齐 |
| 二维码登录 | ✅ | ✅ | 对齐 |
| Welink 登录 | ✅ `Web` + `javaScriptProxy` | ✅ iframe + `hmWelinkJsObj` + `message` | 实现方式不同，能力对齐 |
| 子节点选择 | ✅ `initServerInfoForSysNode` | ❌ 完全缺失 | **能力缺口** |
| License 检查 | ✅ 必须含“鸿蒙客户端授权” | ✅ | 对齐 |
| 服务端版本检查 | 提示要求 `20250905` 以上 | ✅ 已统一为 `buildId >= 20250905` | 已修复 |
| HUB 认证 | 有独立 HUB A/B 登录分支（含硬编码假账号） | ✅ 已排除二维码归类，显示账号密码表单 | 已修复 |
| 隐私模式 | `WindowUtils.setWindowPrivacyModeInPage` | ❌ 无 | 复刻版缺失 |
| 图片 | `startIcon.png`、policy/about/manual | 均已复制 ✅ | 对齐 |

**主要问题**：
1. 复刻版缺失子节点选择、HUB A/B 特殊路径。
2. 原版存在硬编码假账号 TODO、示例 host 等死代码。

---

### 3. 主布局/框架（MainPage ↔ MainLayout）

| 对比项 | 原端 `MainPage.ets` | 复刻端 `MainLayout.vue` | 结论 |
|--------|-------------------|------------------------|------|
| 根结构 | `Tabs` + `Navigation` + 图标 Tab | `grid` + 顶部文字 `<RouterLink>` | 导航形态差异大 |
| 响应式 | 根据 `currentDeviceSize` 切换底部/侧边栏 | 仅小屏调整 tab 宽度 | 复刻版缺少侧边栏模式 |
| 后台任务 | `backgroundTaskManager.startBackgroundRunning` | `clientCommands.startBackgroundTasks()` | 能力依赖 Tauri bridge |
| 系统事件订阅 | USB、网络、锁屏、卸载等公共事件 | 仅订阅 `cpms:hub-*` 业务事件 | 复刻版缺失系统级事件 |
| 打印轮询任务 | `UserJobListManager.runPrintFilesJob` 等 | 未体现具体 interval job | 复刻版缺失 |
| 产品形态 | 区分 `SYS_PRODUCT_C` | 同样区分 `SYS_PRODUCT_C` | 对齐 |
| Tab 图标 | 使用 `ic_home/job_list/job_msg/mine_selected/normal` | ✅ 已使用图标 | 已修复 |
| 路由容器 | `NavPathStack` 支持 printerSelectPage/manualPage 压栈 | 标准 Vue Router | 压栈页面未复刻 |
| 标准版导航 | 首页/设备/我的 | ✅ 已增加“作业列表” | 已补充 |

**主要问题**：复刻版完全移除原生系统事件/后台任务能力；导航无侧边栏；缺失压栈页面。

---

### 4. 首页（HomePage）

| 对比项 | 原端 `HomePage.ets` | 复刻端 `HomePage.vue` | 结论 |
|--------|-------------------|----------------------|------|
| 轮播图 | `Swiper` 组件，多图轮播 | ✅ 已实现自定义轮播（head1/head11/head2/head4） | 已补充 |
| 当前打印机模块 | `HomeCurrentPrinterModule` 独立组件 | 内联 `printer-module` | 合并实现 |
| USB 打印机显示 | 显示名称/类型/UUID，支持复制 UUID | ✅ 显示名称/类型/ID，支持复制 | 已补充 |
| 认证打印 | 条件显示“认证打印”按钮 + 弹窗 | 完全缺失 | **能力缺口** |
| 信息专区 | 多列图片 + 标题/副标题文字叠加 | 三列纯图片卡片 | 信息展示简化 |
| 使用说明 | 拼接 token 打开外部手册 URL | 弹窗展示本地 `use.png` | 行为不一致 |
| 服务状态面板 | 无 | 新增虚拟打印机/Socket/USB 状态网格 | 复刻版新增 |
| 响应式 | `currentDeviceSize` lanes 列数 | CSS `auto-fit` | 基本覆盖 |
| 图片 | `head1/printer0/pic_zone_1/2/3/use` | 均已复制 ✅ | 对齐 |

**主要问题**：复刻版缺失认证打印；原版 `Swiper` 已硬编码为单图但仍保留组件。

---

### 5. 我的/设置页（MinePage）

| 对比项 | 原端 `MinePage.ets` | 复刻端 `MinePage.vue` | 结论 |
|--------|-------------------|----------------------|------|
| 布局 | `Scroll+Column` + 内嵌子视图 | CSS Grid 卡片堆叠 | 结构差异大 |
| 用户信息卡 | 头像 + 用户名/账号 | 纯文本用户名/账号 | 缺少头像 |
| USB 调试面板 | 完整：查看/OPEN/SET/PRINT2/PREHEAT/复制 ID/调速度 | 仅初始化和查看 | **能力大幅裁剪** |
| 服务器检测 | `UserAuthManager.getServerInfo` + `PingTester.pingServer` | `getServerInfo` 仅版本/buildId | 缺失真实 ICMP Ping |
| 指定打印机 | C 版始终显示下拉选择 | 条件显示下拉选择；✅ 已同步服务端 | 已修复 |
| 设置列表 | 动态 `MineListView` | 静态按钮区 | 动态列表丢失 |
| 修改密码 | 内嵌 `PasswordDialog` | 内联弹窗 | 合并实现 |
| 密码校验 | 新密码至少 8 位 | ✅ 已增加 8 位长度校验 | 已修复 |
| 用户手册 | 有入口 | ✅ 已增加使用手册入口 | 已修复 |
| 关于我们/隐私政策 | `WebDialogPageAboutUs/Policy` | 通用 `WebDialog` | 组件形态不同 |
| 退出登录 | 底部固定按钮，清理多存储 | 静态按钮，清理 store + bridge | 基本对齐 |
| 图片 | `ic_account/ic_printer/pic_server` 等 | 未使用 | 资源复制但未引用 |

**主要问题**：USB 调试、Ping 检测、动态设置列表缺失；MinePage 视觉“素化”。

---

### 6. 授权直连打印机页（PrintDeviceListRefreshLoadPageView ↔ PrintDevicePage）

| 对比项 | 原端 `PrintDeviceListRefreshLoadPageView.ets` | 复刻端 `PrintDevicePage.vue` | 结论 |
|--------|--------------------------------------------|---------------------------|------|
| 布局 | `Refresh+List` 卡片，响应式 1/2/3 列 | CSS Grid `auto-fit`，简洁卡片 | 基本对齐，无下拉刷新 |
| 下拉刷新 | ✅ `Refresh` 组件 | ✅ 已增加鼠标/触摸下拉刷新 | 已补充 |
| 设备选择 | Checkbox，选择后 `pop()` 返回并初始化授权 | “设为指定打印机”按钮，保存到客户端 bridge | 行为不一致 |
| 服务端同步 | 选择后调用 `PrintAuthManager` 初始化 | ✅ 已调用 `/cpms/api/userManager/updateDirectDeviceId` | 已修复 |
| 自动返回 | ✅ 选择后 `appPathStack.pop()` | ❌ 无 | 复刻版缺失 |
| 图片 | `print.png` | `print.png` ✅ | 对齐 |

**主要问题**：复刻版选择设备后未自动返回（Web 路由模式不同）。

---

### 7. C 版作业列表页（多文件 ↔ PrintJobPageForC）

| 对比项 | 原端 `hm-client` | 复刻端 `hub-platform` | 结论 |
|--------|-----------------|----------------------|------|
| Tab 结构 | `JobLisTabsPageForC`/`JobListPageForC` + `Tabs` | 顶部按钮切换 | 基本对齐 |
| 打印/复印/扫描 | ✅ 三 tab | ✅ 三 tab | 对齐 |
| 今日/历史筛选 | ✅ 复印/扫描 | ✅ 复印/扫描 | 对齐 |
| 表格/卡片 | 表头来自 `TitleList.ets`，卡片式行带阴影 | 原生 `<table>`，扁平行 | 视觉差异大 |
| 下拉刷新 | ✅ `Refresh` 组件 | ✅ 已增加下拉刷新 | 已补充 |
| 滚动自动加载 | ✅ `List.onScrollIndex` | ✅ 已增加滚动自动加载 | 已补充 |
| 批量取消 | ✅ | ✅ | 对齐 |
| 选择规则 | 仅 `jobStatus==2` 可选（UI 未禁用非 2 状态） | `:disabled="job.jobStatus !== 2"` | 复刻版更规范 |
| 状态标签 | 彩色圆角标签 `JobStatusColors` | ✅ 已增加彩色状态标签 | 已补充 |
| 扫描下载 | 打开外部浏览器 | `clientCommands.openExternal` | 对齐 |
| 响应式 | `currentDeviceSize` 调整 padding/圆角/阴影 | 固定 20px padding | 复刻版缺失 |
| 错误处理 | 加载失败未重置 loading | ✅ 已增加错误状态与重试 | 已修复 |
| 卡片视图 | `JobCardItem`  richer 卡片 | 打印 tab 仍为表格 | **能力部分缺失** |

**主要问题**：复刻版打印 tab 未使用 `JobCardItem` 卡片视图；响应式布局缺失。

---

### 8. 标准版作业列表页（JobLisTabsPage + PrintJobListRefreshLoadPageView ↔ PrintJobPage）

| 对比项 | 原端 `hm-client` | 复刻端 `hub-platform` | 结论 |
|--------|-----------------|----------------------|------|
| 页面存在 | ✅ | ✅ 新增 `PrintJobPage.vue` | 已补充 |
| Tab 结构 | 打印/复印/扫描三 tab | ✅ 三 tab | 已补充 |
| 打印列表 | `JobCardItem` 卡片 | ✅ 使用 `JobCardItem` | 已补充 |
| 复印/扫描 | 表格 | ✅ 表格 | 已补充 |
| 今日/历史筛选 | ✅ 打印/复印/扫描 | ✅ 打印/复印/扫描 | 已补充 |
| 下拉刷新 | ✅ | ✅ | 已补充 |
| 滚动自动加载 | ✅ | ✅ | 已补充 |
| 批量取消 | ✅ | ✅ | 已补充 |
| 状态标签 | 彩色 | ✅ 彩色 | 已补充 |

**结论**：标准版作业列表页已按原端实现补充。

---

### 9. 作业消息页（UserMessagePage ↔ JobMessagePage）

| 对比项 | 原端 `hm-client` | 复刻端 `hub-platform` | 结论 |
|--------|-----------------|----------------------|------|
| 布局 | `Scroll+Column` / `Refresh+List` 卡片 | 扁平 `<table>` + `ListState` | 视觉差异大 |
| 下拉刷新 | ✅（新版 `UserMessageListRefreshLoadPage`） | ✅ 已增加下拉刷新 | 已补充 |
| 手动手势刷新 | `UserMessagePage` 手写 Touch 事件 | ❌ 无 | 复刻版缺失 |
| 分页 | `pageSize=10`（旧）/ `20`（新） | `pageSize=20`，点击/滚动加载 | 对齐 |
| 行样式 | 交替背景色 / 卡片阴影 | ✅ 已增加交替行色 | 已补充 |
| 空状态 | emoji + 多行提示 | `ListState` 纯文本 | 复刻版简化 |
| 错误处理 | `showSuccessMessage(err)` | `ListState type="failure"` 重试 | 复刻版更规范 |
| 图片 | `ic_pull_up_load.gif` | ✅ 已复制 | 已补充 |

**主要问题**：复刻版无手动手势刷新；空状态未使用 emoji。

---

### 10. 公共组件

#### 10.1 CustomTitleBar

| 对比项 | 原端 | 复刻端 | 结论 |
|--------|------|--------|------|
| 品牌区 | `foreground.png` + 标题 | 同左 | 对齐 |
| 背景 | `titleBarBk.png` | 同左 | 对齐 |
| 窗口控制 | 代码保留但 UI 注释 | 完全移除 | 复刻版缺失 |
| 状态显示 | 无 | 打印状态 + Socket 状态 | 复刻版新增 |

#### 10.2 CustomBottomBar

| 对比项 | 原端 | 复刻端 | 结论 |
|--------|------|--------|------|
| 背景 | 透明/深色（注释） | 白色 + 上边框 | 风格差异 |
| 设置入口 | 图标 + “系统设置”文字 | CSS 齿轮图标 + RouterLink | 视觉不一致 |
| 状态显示 | 无 | Socket 连接状态 | 复刻版新增 |

#### 10.3 WebDialog

| 对比项 | 原端 | 复刻端 | 结论 |
|--------|------|--------|------|
| 实现 | 三个独立 `@CustomDialog` | 一个通用组件 | 复刻版更精简 |
| 模式 | 仅 Web | Web + Image | 复刻版扩展 |
| 资源 | policy/about/manual html | 均已复制 ✅ | 对齐 |

#### 10.4 ListState（合并 Failure/Loading/LoadMore/NoMore/RefreshLayout）

| 对比项 | 原端 | 复刻端 | 结论 |
|--------|------|--------|------|
| 功能 | 多个独立布局组件 | 一个统一状态组件 | 合并合理 |
| 加载动画 | 系统 `LoadingProgress` | CSS spinner | 视觉不一致 |

#### 10.5 JobCardItem

| 对比项 | 原端 `JobCardItem.ets` | 复刻端 `JobCardItem.vue` | 结论 |
|--------|----------------------|-------------------------|------|
| 存在 | ✅ | ✅ 已新增 | 已补充 |
| 文件徽标 | 按扩展名取色块 | ✅ 按扩展名取色块 | 已补充 |
| 文件名省略 | ✅ | ✅ | 已补充 |
| 状态标签 | 彩色圆角 | ✅ 彩色圆角 | 已补充 |
| 参数 chip | 份数/彩色/双面/页数 | ✅ | 已补充 |
| 悬停放大 | ✅（LG 屏） | ✅（≥1024px） | 已补充 |

---

## 三、原端未复刻的页面/视图/组件

以下原端文件在复刻端**没有独立对应实现**（部分功能已合并）：

### 3.1 页面/视图（建议补充或评估是否必要）

| 序号 | 原端文件 | 说明 | 状态 |
|------|---------|------|------|
| 1 | `view/JobLisTabsPage.ets` | 非 C 版作业 Tab 容器 | ✅ 已合并到 `PrintJobPage.vue` |
| 2 | `view/JobListPageC.ets` | C 端桌面原型页，硬编码数据 | 🟢 可忽略 |
| 3 | `view/PrintJobListRefreshLoadPageView.ets` | 旧版打印作业列表（使用 `JobCardItem`） | ✅ 已合并到 `PrintJobPage.vue` |
| 4 | `view/JobListRefreshLoadPageViewCopy.ets` | 复印作业列表 | ✅ 已合并到 `PrintJobPage.vue` |
| 5 | `view/JobListRefreshLoadPageViewScan.ets` | 扫描作业列表 | ✅ 已合并到 `PrintJobPage.vue` |
| 6 | `view/ListRefreshLoadPage.ets` | 通用列表状态组件 | ✅ 已合并到 `ListState.vue` |
| 7 | `view/EmptyPagePathStack.ets` | 占位页 | 🟢 可忽略 |
| 8 | `view/ManualPagePathStack.ets` | 手册占位页 | 🟢 可忽略 |
| 9 | `view/PrinterSelectPagePathStack.ets` | 压栈式打印机选择页 | 🟡 路由模式不同，可用 `/devices` 替代 |
| 10 | `view/TestPage.ets` | 开发调试页 | 🟢 可忽略 |
| 11 | `view/MyPageTest.ets` | 测试页 | 🟢 可忽略 |

### 3.2 组件

| 序号 | 原端文件 | 说明 | 状态 |
|------|---------|------|------|
| 1 | `biz/components/JobCardItem.ets` |  richer 作业卡片组件 | ✅ 已补充 |

### 3.3 可忽略的空/测试文件

| 原端文件 | 大小/行数 | 说明 |
|---------|----------|------|
| `view/PrinterAddPage.ets` | 0 字节 | 空文件，无实现 |
| `view/MyPageTest.ets` | 58 行 | Hello World 测试页 |
| `view/TestPage.ets` | 405 行 | SM4/签名/文件扫描测试页 |
| `view/EmptyPagePathStack.ets` | 78 行 | 占位页 |
| `view/ManualPagePathStack.ets` | 78 行 | 占位页 |

---

## 四、图片资源完整性

- 原端 `entry/src/main/resources/base/media/`：83 个文件
- 复刻端 `src/assets/images/`：83 个文件
- **缺失 0 个**：
  - ✅ `ic_pull_up_load.gif` 已补充
  - ✅ `layered_image.json` 已补充

其余 81 个图片文件大小完全一致，确认直接复制。

**注意**：大量已复制的图标（如 `ic_account/ic_printer/pic_server/ic_about/ic_policy` 等）在复刻版页面中并未引用，处于“资源在但页面未使用”状态。

---

## 五、不规范与 Bug 汇总

### 5.1 复刻端（hub-platform）已修复

| 问题 | 修复位置 |
|------|---------|
| HUB 认证被错误归类为二维码认证 | `src/stores/auth.ts` `isQrCodeAuth` |
| 登录页缺失自动登录 | `src/pages/LoginPage.vue` `onMounted` |
| 服务端版本阈值不一致 | `src/stores/server.ts` `20260506 → 20250905` |
| 设备选择后未同步服务端 | `src/services/api/printer.api.ts` + `src/stores/print.ts` |
| 作业列表无错误处理 | `src/stores/job.ts` `errorMessage` |
| 作业状态丢失彩色标签 | `src/pages/PrintJobPageForC.vue` / `PrintJobPage.vue` |
| 首页缺失轮播图 | `src/pages/HomePage.vue` 自定义轮播 |
| USB UUID 无法复制 | `src/pages/HomePage.vue` / `MinePage.vue` |
| 消息页无下拉刷新 | `src/pages/JobMessagePage.vue` |
| 我的页无用户手册入口 | `src/pages/MinePage.vue` `openManual` |
| 密码校验无长度要求 | `src/pages/MinePage.vue` 新密码 ≥ 8 位 |
| MainLayout Tab 无图标 | `src/pages/MainLayout.vue` |
| 缺失图片资源 | `src/assets/images/` 已补齐 |

### 5.2 复刻端（hub-platform）仍存在的缺口

| 优先级 | 问题 | 说明 |
|--------|------|------|
| 🟡 中 | 子节点选择完全缺失 | 需按 IP 段匹配子节点，逻辑复杂 |
| 🟡 中 | 首页认证打印能力 | 需 Tauri 客户端桥扩展 |
| 🟡 中 | MinePage USB 调试面板 | OPEN/SET/PRINT2/PREHEAT 等需客户端桥 |
| 🟡 中 | MinePage 真实 Ping 检测 | `PingTester.pingServer` 为原生能力 |
| 🟢 低 | 响应式栅格/设备尺寸断点 | 视觉简化 |
| 🟢 低 | C 版打印 tab 未使用 `JobCardItem` | 当前为表格，与原版卡片视图有差异 |
| 🟢 低 | 消息页空状态无 emoji | 视觉简化 |

### 5.3 原端（hm-client）可优化

| 优先级 | 问题 | 位置/说明 |
|--------|------|----------|
| 🟡 中 | `LoginPage.ets` 存在 HUB A 硬编码假账号 | `doAuthHubLogin` 固定 `userId=99999` |
| 🟡 中 | `LoginPage.ets` `cpmsServerVersionValid` 初始化为 `true`（//TODO） | 第 350 行 |
| 🟡 中 | `LoginPage.ets` Welink JSProxy 示例 host 未清理 | `getPermission()` 含 `xxx.com` |
| 🟡 中 | `PolicyPage.ets` `PolicyBottom` 死代码 | 被注释但仍保留 200+ 行 |
| 🟡 中 | 打印作业列表复选框状态被覆盖 | `PrintJobListRefreshLoadPageViewForC.ets` `.select(...).select(false)` |
| 🟡 中 | 加载失败未重置 loading 状态 | 多处 `catch` 只弹提示 |
| 🟢 低 | 错误提示误用 `showSuccessMessage` | 多处 `PromptUtils.showSuccessMessage(err)` |
| 🟢 低 | `PrinterAddPage.ets` 为空文件 | 未实现 |
| 🟢 低 | `PasswordDialogPage.ets` 独立存在但未被使用 | MinePage 内嵌版才是实际弹窗 |

---

## 六、补充完成情况

### 6.1 新增文件

| 文件 | 说明 |
|------|------|
| `hub-platform/src/utils/jobStatus.ts` | 作业状态颜色工具 |
| `hub-platform/src/utils/fileIcon.ts` | 文件图标颜色与类型简称 |
| `hub-platform/src/components/common/JobCardItem.vue` | 作业卡片组件 |
| `hub-platform/src/pages/PrintJobPage.vue` | 标准版作业列表页 |

### 6.2 修改文件

| 文件 | 修改内容 |
|------|---------|
| `hub-platform/src/stores/auth.ts` | HUB 认证排除二维码归类 |
| `hub-platform/src/stores/server.ts` | 版本阈值统一为 20250905 |
| `hub-platform/src/stores/job.ts` | 增加错误状态 |
| `hub-platform/src/stores/print.ts` | 选择打印机同步服务端 |
| `hub-platform/src/services/api/printer.api.ts` | 新增 `selectDirectDevice` |
| `hub-platform/src/pages/LoginPage.vue` | 自动登录、自动加载认证方式 |
| `hub-platform/src/pages/HomePage.vue` | 轮播图、USB ID 复制 |
| `hub-platform/src/pages/PrintDevicePage.vue` | 下拉刷新、服务端同步、错误提示 |
| `hub-platform/src/pages/PrintJobPageForC.vue` | 下拉刷新、滚动加载、状态色、错误重试 |
| `hub-platform/src/pages/JobMessagePage.vue` | 下拉刷新、滚动加载、交替行色 |
| `hub-platform/src/pages/MinePage.vue` | 使用手册入口、密码长度校验 |
| `hub-platform/src/pages/MainLayout.vue` | Tab 图标、标准版作业列表入口 |
| `hub-platform/src/router/index.ts` | 注册 `/jobs` 路由 |

### 6.3 资源补充

| 文件 | 说明 |
|------|------|
| `hub-platform/src/assets/images/ic_pull_up_load.gif` | 从原端复制 |
| `hub-platform/src/assets/images/layered_image.json` | 从原端复制 |

### 6.4 验证结果

- ✅ `pnpm build` 通过
- ✅ `pnpm lint` 通过（0 warnings, 0 errors）

---

## 七、总体结论

### 7.1 复刻完成度

- **核心页面覆盖**：隐私政策、登录、主框架、首页、我的、设备、标准版/C 版作业列表、作业消息均已实现。
- **视觉还原度**：中上。标题栏、banner、打印机卡片、信息区图片、Tab 图标、状态色已对齐；但底部栏风格、卡片阴影、响应式栅格仍有差异。
- **功能完整性**：中高。标准版作业列表、`JobCardItem`、下拉刷新、滚动加载、状态色、服务端同步、USB UUID 复制、轮播图、用户手册等已补充；系统级后台任务/事件订阅、USB 调试面板、真实 Ping、认证打印等强原生能力仍受限于客户端桥。
- **资源完整性**：图片复制率 100%（83/83）。

### 7.2 验收结论

| 检查项 | 状态 |
|--------|------|
| 隐私政策页对比 | ✅ 已完成 |
| 登录页对比 | ✅ 已完成 |
| 主布局/框架对比 | ✅ 已完成 |
| 首页对比 | ✅ 已完成 |
| 我的页对比 | ✅ 已完成 |
| 授权直连打印机页对比 | ✅ 已完成 |
| C 版作业列表页对比 | ✅ 已完成 |
| 标准版作业列表页对比 | ✅ 已完成 |
| 作业消息页对比 | ✅ 已完成 |
| 公共组件（TitleBar/BottomBar/WebDialog/ListState/JobCardItem）对比 | ✅ 已完成 |
| 缺失页面/组件/图片清单 | ✅ 已列出 |
| 不规范与 Bug 清单 | ✅ 已列出 |
| 缺失内容 Web 端补充 | ✅ 已完成 |
| 构建与 lint 校验 | ✅ 通过 |

**结论**：本次已按“Web 端优先补充”方案完成 `hub-platform` 相对 `hm-client` 的主要缺失补充，并通过构建与 lint 校验。剩余缺口多为强原生能力或复杂业务逻辑，如需进一步补齐需扩展 Tauri 客户端桥。
