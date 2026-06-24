# CPMS Client 设计规范

> 基于 `ui-ux-pro-max` Skill 制定的统一设计系统：Fresh Minimal / Enterprise Gateway。
> 本文档描述 CPMS Client 桌面宿主端的 UI/UX、公共组件和页面规范。技术前提：基础样式与组件体系统一切换为 TailwindCSS + Shadcn-UI（Vue 项目采用 shadcn-vue / Reka UI 实现）。

## 1. 项目定位

- **产品类型**：企业级打印客户端宿主，负责桌面外壳、iframe 容器、通知窗口、调试面板和宿主桥接。
- **目标平台**：Windows / macOS / Linux，当前客户端原生能力只维护 `client-tauri1`。
- **设计语言**：权威、专业、紧凑、可扫描、可信赖。
- **交互密度**：桌面工具型高密度，主要面向鼠标键盘，同时保证 44px 以上可点击区域。
- **跨端关系**：`client` 是容器与系统能力层，`hub-web` 是 iframe 内业务视图；两端共享色彩、字体、圆角、阴影和动效节奏，但组件按各自职责拆分。

## 2. 技术基座

### 2.1 UI 技术栈

- 使用 **TailwindCSS** 作为样式基础，负责布局、间距、状态、响应式和主题变量消费。
- 使用 **Shadcn-UI Vue 体系**作为公共组件基座，组件源码落在 `client-ui/src/components/ui`，按需复制、可本地改造。
- 使用 **Reka UI / Radix Vue 类无样式原语**承载 Dialog、Tabs、Tooltip、Popover、Select 等复杂交互语义。
- 使用 **@lucide/vue** 作为结构图标库，保持 1.75px 或 2px stroke 的一致风格。
- 不再引入旧组件库、旧图标包、旧原子样式方案或依赖 `i-*` 原子图标类。

### 2.2 样式边界

- 设计令牌集中在 `client-ui/src/assets/styles/tokens.css` 与 Tailwind theme 中维护。
- Tailwind 负责页面组合层；复杂组件样式优先沉淀到 shadcn 组件变体，不在页面里堆叠大量一次性 class。
- `--cpms-*` 作为宿主语义令牌继续保留，迁移期映射到 shadcn 的语义变量，避免宿主与 iframe 视觉断层。
- 禁止在组件中直接写裸色值、随机阴影和临时字号；新增颜色必须先进入令牌。

## 3. 设计令牌

### 3.1 Shadcn 语义变量

以浅色工作台为默认主题，暗色主题可后续补齐。CSS 变量建议使用 HSL 三元值，便于 Tailwind `hsl(var(--token))` 消费。

```css
:root {
  --background: 210 40% 98%;
  --foreground: 222 47% 11%;
  --card: 0 0% 100%;
  --card-foreground: 222 47% 11%;
  --popover: 0 0% 100%;
  --popover-foreground: 222 47% 11%;
  --primary: 224 64% 33%;
  --primary-foreground: 0 0% 100%;
  --secondary: 210 40% 96%;
  --secondary-foreground: 222 47% 11%;
  --muted: 210 40% 96%;
  --muted-foreground: 215 16% 47%;
  --accent: 200 95% 32%;
  --accent-foreground: 0 0% 100%;
  --destructive: 0 72% 51%;
  --destructive-foreground: 0 0% 100%;
  --border: 214 32% 91%;
  --input: 214 32% 91%;
  --ring: 224 64% 33%;
  --radius: 0.5rem;
}
```

### 3.2 宿主别名令牌

```css
:root {
  --cpms-color-bg-app: hsl(var(--background));
  --cpms-color-bg-panel: hsl(var(--card));
  --cpms-color-text-primary: hsl(var(--foreground));
  --cpms-color-text-secondary: #334155;
  --cpms-color-text-muted: hsl(var(--muted-foreground));
  --cpms-color-border: hsl(var(--border));
  --cpms-color-foreground: #0f172a;
  --cpms-color-primary: hsl(var(--primary));
  --cpms-color-accent: hsl(var(--accent));
  --cpms-color-danger: hsl(var(--destructive));
}
```

### 3.3 色彩策略

| 用途 | 颜色 | 说明 |
| --- | --- | --- |
| 主色 | `#2563EB` | 主按钮、焦点环、选中态 |
| 强调色 | `#059669` | 次级 CTA、完成态、辅助重点操作 |
| 背景 | `#F8FAFC` | 主窗口内容区、iframe 外壳背景 |
| 面板 | `#FFFFFF` | 卡片、抽屉、通知窗口内容面 |
| 文本 | `#020617` / `#334155` / `#64748B` | 主文本、次级文本、辅助文本 |
| 边框 | `#E2E8F0` | 面板、输入框、分隔线 |
| 危险 | `#DC2626` | 关闭、清空日志、危险确认 |

### 3.4 字体、间距、圆角

- 字体：`Plus Jakarta Sans, Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif`。
- 基础字号：16px；正文行高 1.5；标题字重 600-700。
- 间距：4px 网格，Tailwind spacing 使用 `1=4px`、`2=8px`、`3=12px`、`4=16px`。
- 圆角：shadcn 默认 `--radius: 8px`；宿主工具按钮 6px，面板 8px，抽屉和通知窗口外框 12px。
- 阴影：只保留 `shadow-sm`、`shadow-md`、`shadow-lg` 三档，禁止卡片过度漂浮。

### 3.5 动效

- hover / pressed：150ms。
- Dialog、Drawer、Tooltip、Tabs 内容切换：200-300ms。
- 只动画 `opacity` 与 `transform`，不动画宽高。
- 必须支持 `prefers-reduced-motion: reduce`，减少或关闭过渡。

## 4. 公共组件

公共组件分两层：`components/ui` 是 shadcn 基础组件，`components/common` / `components/layout` 是 CPMS 语义组件。

### 4.1 基础组件清单

| 组件 | 来源 | 用途 |
| --- | --- | --- |
| `Button` | shadcn-vue | 主按钮、次按钮、危险按钮、图标按钮 |
| `Input` / `Textarea` | shadcn-vue | 服务器地址、调试输入、日志筛选 |
| `Label` | shadcn-vue | 所有表单可见标签 |
| `Dialog` / `AlertDialog` | shadcn-vue | 关闭确认、清空日志确认、错误详情 |
| `Sheet` | shadcn-vue | 调试面板右侧抽屉 |
| `Tabs` | shadcn-vue | 能力检测 / 客户端日志 |
| `Tooltip` | shadcn-vue | 标题栏图标按钮说明 |
| `Alert` | shadcn-vue | 错误提示、桥接异常提示 |
| `ScrollArea` | shadcn-vue | 日志、调试结果长内容滚动 |
| `Badge` | shadcn-vue | 连接状态、通知类型、最近地址 |
| `Skeleton` | shadcn-vue | iframe 初始化、日志加载占位 |

### 4.2 语义组件清单

| 组件 | 路径建议 | 职责 |
| --- | --- | --- |
| `WindowFrame` | `components/layout` | 无边框窗口根容器，统一背景、圆角、阴影和溢出裁剪 |
| `WindowHeaderBar` | `components/layout` | 自绘标题栏、拖拽区、窗口控制按钮 |
| `HostStatusBadge` | `components/common` | 连接、socket、token、serverAddress 等宿主状态 |
| `ErrorNotice` | `components/common` | shadcn `Alert` 封装，展示标题、描述、恢复动作 |
| `DebugSheet` | `components/layout` | 右侧调试抽屉，内部承载 Tabs |
| `LogViewer` | `components/common` | 日志级别、复制、清空、滚动定位 |
| `BridgeResultBlock` | `components/common` | 能力检测结果 JSON/文本块，支持复制 |
| `IframeFallback` | `components/common` | iframe 空地址、加载失败、刷新重试 |

## 5. 宿主页面规范

### 5.1 主窗口 `HomeView`

- 骨架：`WindowFrame` -> `WindowHeaderBar` -> iframe 内容区 -> 右下角调试入口。
- iframe 内容区默认铺满剩余空间，不额外包卡片，避免业务页面被双重框住。
- 调试入口使用 44x44 图标按钮，固定右下角，图标为 `Bug` 或 `PanelRightOpen`，带 Tooltip。
- iframe 加载中超过 300ms 显示轻量 skeleton；加载失败显示 `IframeFallback`，提供重试和回到入口页。

### 5.2 入口页 `EntryView`

- 使用居中窄面板，最大宽度 520px，面板半径 12px，边框 `border`，阴影 `shadow-md`。
- 表单字段必须有可见 `Label`，地址输入使用 `Input type="url"`，支持 Enter 提交。
- 主 CTA 使用 `Button` 的 `default` 变体，高度 44px 或 48px。
- 最近地址使用 `Badge` + 图标按钮，不使用胶囊文本模拟按钮。
- 错误信息位于字段下方，使用 `aria-describedby` 关联输入。

### 5.3 调试面板

- 用 shadcn `Sheet` 从右侧进入，宽度 `min(80vw, 900px)`，最小可用宽度 560px。
- 顶部为面板标题和关闭按钮；内容用 `Tabs` 切分能力检测与客户端日志。
- 能力检测模块使用紧凑面板，每个模块包含标题、说明、操作按钮、结果块。
- 结果块使用 `ScrollArea` + `BridgeResultBlock`，代码字体 13px，复制按钮为图标按钮。

### 5.4 客户端日志页

- 顶部工具栏：级别筛选、刷新、复制、清空。
- 清空日志必须使用 `AlertDialog` 二次确认。
- 日志正文使用等宽字体、保留换行、按级别增加文本标识，不只靠颜色区分。
- 日志加载或刷新超过 300ms 显示 skeleton 或按钮 loading 状态。

### 5.5 通知窗口 `NotificationView`

- 通知窗口保持 400x400，外层透明，内容面使用 `WindowFrame`。
- 顶部复用 `WindowHeaderBar`，正文使用类型图标、标题、消息文本和可选操作。
- error/warning/success/info 使用 `Alert` 风格映射，状态必须有图标与文本。
- 关闭按钮为 44x44 图标按钮，带 `aria-label`。

## 6. 交互规范

- 所有 icon-only 按钮必须有 `aria-label` 和 Tooltip。
- 按钮异步提交时必须禁用并显示 loading 状态，避免重复提交。
- 关闭、清空、退出、覆盖配置等危险动作必须二次确认。
- Keyboard：Tab 顺序与视觉顺序一致；Dialog/Sheet 打开后焦点进入容器，关闭后回到触发器。
- 状态提示：请求失败必须说明原因和恢复路径，例如“重试”“回到入口页”“复制错误”。
- Toast 不作为唯一错误承载；关键错误需要页面内 Alert。

## 7. 可访问性与质量门槛

- 普通文本对比度不低于 4.5:1，大号文本不低于 3:1。
- 焦点环使用 `ring-2 ring-primary ring-offset-2`，不得移除。
- 可点击目标 hit area 不低于 44x44px。
- 不使用 emoji 作为结构图标；统一 lucide 图标。
- 支持 125% / 150% 系统缩放，文字不得被按钮或卡片裁切。
- 支持 reduced motion。

## 8. 与 hub-web 的跨端契约

| 维度 | client | hub-web iframe |
| --- | --- | --- |
| 角色 | 桌面外壳、系统能力、容器 | 登录与业务流程 |
| 主色 | `#2563EB` | `#2563EB` |
| 强调色 | `#059669` | `#059669` |
| 背景 | iframe 外壳 `#F8FAFC` | 页面背景 `#F8FAFC` |
| 字体 | Plus Jakarta Sans / Inter | Plus Jakarta Sans / Inter |
| 组件库 | shadcn-vue + Tailwind | shadcn-vue + Tailwind |
| 图标 | @lucide/vue | @lucide/vue |

client 不复制业务页面样式；iframe 内外只共享设计令牌和状态语义。业务导航、业务表格、业务弹窗全部属于 `hub-web`。

## 9. 迁移原则

1. 先建立 Tailwind 与 shadcn 基础设施，再替换页面组件。
2. 先迁移公共组件，再迁移页面：`WindowHeaderBar`、`ErrorNotice`、`DebugSheet`、`LogViewer` 优先。
3. 旧组件对应关系：
   - `el-button` -> `Button`
   - `el-input` -> `Input`
   - `el-alert` -> `Alert`
   - `el-drawer` -> `Sheet`
   - `el-tabs` / `el-tab-pane` -> `Tabs`
   - `ElMessageBox` -> `AlertDialog`
   - `ElMessage` -> `Toast` 或页面内 `Alert`
4. 旧工具类迁移到 Tailwind class；图标类迁移到 lucide 组件。
5. 移除 EP/Uno 后再清理 Vite 插件、自动引入配置和类型白名单。

## 10. 页面速查

- **HomeView**：无卡片 iframe，全屏承载，右下角调试入口。
- **EntryView**：单卡片配置表单，主 CTA 明确，最近地址可快速复用。
- **DebugSheet**：右侧抽屉，Tabs 分区，结果可复制。
- **LogView**：工具栏 + 滚动日志区，危险清空二次确认。
- **NotificationView**：400x400 独立通知窗口，类型图标 + 消息 + 关闭。

---

_本文档更新于 2026-06-24，用于指导 TailwindCSS + Shadcn-UI 迁移。_
