# CPMS Client 设计规范

> 基于 `ui-ux-pro-max` Skill 对 client-ui 全部页面与组件的梳理总结。
> 本文档描述 CPMS Client（PC 桌面端）当前真实的设计实现，并给出与 hub-platform（平板/Web 端）统一风格的方案。

## 1. 项目概述

- **产品类型**：企业级打印客户端基座（PC 桌面端）。
- **技术栈**：Vue 3 + TypeScript + Vite + Element Plus + UnoCSS，Tauri 1/2 作为桌面壳。
- **目标平台**：Windows / macOS / Linux（含国产 Linux legacy）。
- **窗口形态**：
  - 主窗口：默认 800×600，无系统标题栏（`decorations: false`），自绘 `WindowHeaderBar`。
  - 通知子窗口：400×400，屏幕右下角，自绘标题栏。
  - 调试抽屉：从主窗口右侧滑出，宽度 80%。
- **设计语言**：紧凑、专业、清爽，以 Element Plus 为基础，通过 `--cpms-*` 令牌对齐自有视觉。

## 2. 设计语言与风格

### 2.1 整体风格

- **风格关键词**：专业、紧凑、工具型、无边框窗口。
- **视觉层次**：通过自绘标题栏、面板背景色、Element Plus 组件层级区分。
- **密度**：高密度，适合 PC 端鼠标操作与信息密度较大的调试/日志场景。
- **色彩倾向**：以 Element Plus 主色蓝（`#2354f4`）为主，灰阶文本与边框，红色为危险态。

### 2.2 与 ui-ux-pro-max 的契合点

| Skill 规则 | 当前实现 | 符合度 |
|---|---|---|
| `style-match`：产品类型匹配 | 企业工具型桌面客户端 | ✅ 高 |
| `consistency`：全站风格一致 | 全部窗口共用 `tokens.css` | ✅ 高 |
| `no-emoji-icons`：不使用 emoji | 使用 SVG + Element Plus 图标 | ✅ 高 |
| `system-controls`：优先系统/成熟组件 | 使用 Element Plus | ✅ 高 |
| `elevation-consistent`：阴影统一 | 基本依赖 Element Plus 阴影，自定义较少 | ⚠️ 中等 |

## 3. 设计令牌（Design Tokens）

所有令牌集中在 `client-ui/src/assets/styles/tokens.css` 的 `:root` 中。

### 3.1 颜色系统

```css
/* 文本 */
--cpms-color-text-primary: #1f2937;
--cpms-color-text-secondary: #4b5563;
--cpms-color-text-muted: #6b7280;

/* 背景 */
--cpms-color-bg-app: #f4f6f8;
--cpms-color-bg-panel: #ffffff;
--cpms-color-bg-hover: #f1f4f8;
--cpms-color-bg-code: #f4f6f8;

/* 边框 */
--cpms-color-border: #e5e7eb;

/* 语义色 */
--cpms-color-danger: #dc2626;
--cpms-color-danger-bg: #fef2f2;
```

**对比度检查（基于 WCAG AA）**：
- `#1f2937` on `#ffffff`：约 11.5:1 ✅
- `#4b5563` on `#ffffff`：约 7.4:1 ✅
- `#6b7280` on `#ffffff`：约 5.4:1 ✅
- `#2354f4` on `#ffffff`：约 5.4:1 ✅

### 3.2 字体系统

```css
--cpms-font-family: Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif;
--cpms-font-size-base: 14px;
--cpms-font-size-small: 13px;
--cpms-font-size-title: 14px;
--cpms-line-height-base: 22px;
--cpms-line-height-small: 20px;
```

| 用途 | 字号 | 行高 | 颜色 |
|---|---|---|---|
| 窗口标题栏文字 | 14px | 20px (`--cpms-line-height-small`) | `--cpms-color-text-primary` |
| 正文/标签 | 14px | 22px (`--cpms-line-height-base`) | `--cpms-color-text-primary` |
| 辅助说明 | 13px | 20px (`--cpms-line-height-small`) | `--cpms-color-text-secondary` |
| 代码/日志 | 13px | 20px (`--cpms-line-height-small`) | `--cpms-color-text-primary` |

**建议改进**：
- 当前字号系统较简单（仅 base/small/title），建议补充 `large`（16px）与 `tiny`（12px）。
- 标题栏字号与正文相同，层级感不足，建议标题栏使用 15px 或中等字重区分。

### 3.3 间距系统

```css
--cpms-space-xs: 4px;
--cpms-space-small: 8px;
--cpms-space-base: 12px;
--cpms-space-large: 16px;
--cpms-space-xlarge: 20px;
```

**使用规则**：
- `xs`：图标与文字间隙、标题栏按钮间距。
- `small`：卡片内部小间隙、按钮组间距。
- `base`：卡片 padding、抽屉内容区 padding。
- `large`：模块/区块间距、抽屉页签头部 padding。
- `xlarge`：悬浮按钮/窗口边缘的大间距。

**建议改进**：
- PC 端 spacing 已扩展为 4/8/12/16/20 五档，与 hub-platform 的 6/10/16/20/24 档位对齐，仅密度不同。

### 3.4 圆角系统

```css
--cpms-radius-small: 6px;
--cpms-radius-panel: 8px;
--cpms-radius-medium: 10px;
--cpms-radius-large: 16px;
```

| 用途 | 圆角 |
|---|---|
| 按钮/输入框/小标签/标题栏按钮 | `small` (6px) |
| 卡片/面板/警告条 | `panel` (8px) |
| 抽屉/大面板左侧圆角 | `large` (16px) |

**建议改进**：
- 已引入 10px（medium）与 16px（large）圆角，与 hub-platform 的 md/lg 对齐。

### 3.5 阴影系统

```css
--cpms-shadow-sm: 0 2px 8px rgba(15, 23, 42, 0.04);
--cpms-shadow-md: 0 4px 16px rgba(15, 23, 42, 0.06);
--cpms-shadow-lg: 0 8px 24px rgba(15, 23, 42, 0.1);
```

| 用途 | 阴影 |
|---|---|
| 卡片/代码块 | `sm` |
| 通知子窗口 | `md` |
| 调试抽屉 | `lg` |

**建议改进**：
- 已定义与 hub-platform 对齐的三级阴影，并在抽屉、通知卡片、调试卡片上应用。

## 4. 组件规范

### 4.0 UnoCSS 与 Element Plus 应用约定

- **Element Plus 是交互组件基座**：按钮、抽屉、页签、提示、空状态、确认框、loading 指令等优先使用 Element Plus，避免在业务页重复手写成熟组件行为。
- **Element Plus 样式通过令牌对齐**：`client-ui/src/assets/styles/tokens.css` 用 `:root:root` 覆盖关键 `--el-*` 变量，使 `el-button`、`el-input`、`el-tag`、`el-drawer`、`el-tabs`、`el-alert` 与 `--cpms-*` 视觉令牌保持一致。
- **UnoCSS 用于原子化辅助样式**：项目通过 `client-ui/uno.config.ts` 启用 `presetUno`、`presetAttributify`、`presetIcons`、`presetTypography`、`presetWebFonts` 与 directives / variant group transformer；适合补充布局、间距、状态类和轻量工具类。
- **样式优先级**：跨组件视觉规范写入 `tokens.css`；单组件结构样式写在对应 `.vue` 的 scoped CSS；局部、一次性布局可用 UnoCSS。不要用 UnoCSS 绕过设计令牌写散乱裸色值。
- **图标使用**：Vite 已接入 `unplugin-icons` 与 `IconsResolver`，Element Plus 图标集合可按 `i-ep-*` / 自动注册组件方式使用；窗口控制等已有内联 SVG 可保持不变。

### 4.1 窗口标题栏（`WindowHeaderBar`）

```text
┌────────────────────────────────────────┐
│  [Logo] 标题                    ─ □ ✕ │
└────────────────────────────────────────┘
```

- **高度**：`var(--cpms-headerbar-height)` = 44px。
- **背景**：`--cpms-color-bg-panel`。
- **底部边框**：1px `--cpms-color-border`。
- **标题**：左侧 Logo（18px）+ 文字，可拖拽区（`data-tauri-drag-region`）。
- **控制按钮**：28×28px，图标 14px， hover 背景色反馈。
  - 固定/取消固定：pin 图标，`is-active` 时主色。
  - 收起：下划线图标。
  - 全屏/退出全屏：全屏图标。
  - 关闭：× 图标，hover 危险红底。

**可访问性**：
- 所有图标按钮均带 `aria-label` 与 `title` ✅。
- 关闭按钮 hover 使用危险色，视觉反馈明确 ✅。

**建议改进**：
- 控制按钮 28×28px，略小于 44×44 建议值，但 PC 端鼠标操作可接受。
- 标题栏与内容区对比度良好，但缺少 focus 状态样式。

### 4.2 主窗口布局（`HomeView`）

```text
┌─────────────────────────┐
│     WindowHeaderBar     │
├─────────────────────────┤
│                         │
│      iframe 业务页       │
│                         │
│  [调试] 浮动按钮          │
└─────────────────────────┘
```

- 主内容区为全屏 iframe，加载 hub-platform。
- 右下角固定“调试”按钮（`el-button type="primary"`），点击打开调试抽屉。
- 抽屉宽度 80%，内部使用 `el-tabs` 分两页：能力检测 / 客户端日志。

**建议改进**：
- “调试”按钮在正式产品中应可配置隐藏或降级为更小入口。
- 抽屉宽度已由 `size="80%"` 改为受 CSS 约束：`min-width: 560px`、`max-width: 900px`，兼顾小窗与大屏。

### 4.3 调试能力检测页（`ExampleView`）

- **布局**：单列卡片流，每个 `section.card` 一个能力模块。
- **卡片样式**：
  - 背景 `--cpms-color-bg-panel`
  - 边框 1px `--cpms-color-border`
  - 圆角 `--cpms-radius-panel` (8px)
  - padding `--cpms-space-base` (12px)
  - 内部 gap `--cpms-space-small` (8px)
- **按钮**：主要使用 `el-button`，type 包含 primary / success / warning / danger / plain。
- **结果展示**：`<pre class="result">`，浅灰背景 `--cpms-color-bg-code`。

**建议改进**：
- 卡片间距统一为 `base`（12px），标题区与内容区已拆分为 `h2` + `.card-body`。
- ✅ 结果代码块已增加“复制”按钮，支持一键复制检测结果。
- ✅ 各能力模块已支持折叠/展开，降低信息密度。

### 4.4 客户端日志页（`LogView`）

- **布局**：工具栏 → 文件状态 → 日志文本区。
- **工具栏**：左侧类别下拉（160px），右侧刷新/复制/清空按钮。
- **文件状态**：浅灰背景条，显示日志文件路径与大小。
- **日志文本区**：`<pre>`，最大高度 60vh，浅灰背景，自动换行。

**优点**：
- 日志按类别筛选 ✅。
- 全量长文本展示，不截断 ✅。
- 空状态使用 `el-empty` ✅。

**建议改进**：
- 日志文本区 60vh 在抽屉中可能过矮，建议改为 `height: calc(100% - 工具栏高度)` 撑满剩余空间。
- 缺少日志级别颜色区分（error/warn/info）。
- ✅ 清空操作已增加 `ElMessageBox.confirm` 二次确认。

### 4.5 通知子窗口（`NotificationView`）

- **尺寸**：400×400（由 Tauri 创建）。
- **布局**：自绘标题栏 + 通知正文。
- **视觉**：卡片式窗口，背景 `--cpms-color-bg-panel`，已增加 `box-shadow: var(--cpms-shadow-md)`。
- **交互**：点击关闭按钮隐藏窗口（非退出）。

**建议改进**：
- 通知正文区 padding 为 12px，适合桌面通知密度。
- ✅ 已根据 type 显示左侧色条 + 类型图标（信息/成功/警告/错误）与对应颜色。
- ✅ 已添加 `box-shadow: var(--cpms-shadow-md)`。

### 4.6 错误提示（`ErrorNotice`）

- 使用 `el-alert` 组件。
- 类型：`error` / `warning`。
- 显示标题 + 来源/错误码描述。
- 可关闭。

**建议改进**：
- 当前错误提示位于调试页顶部，主窗口 iframe 内错误由 hub-platform 自身处理，分工清晰 ✅。
- 建议增加错误发生时间戳。

## 5. 布局规范

### 5.1 页面骨架

```text
App.vue
├── HomeView（主窗口）
│   ├── WindowHeaderBar
│   ├── main.iframe-root
│   │   ├── iframe（hub-platform）
│   │   └── el-button.debug-trigger
│   └── el-drawer.debug-drawer
│       ├── WindowHeaderBar（抽屉标题）
│       └── el-tabs
│           ├── ExampleView
│           └── LogView
└── NotificationView（通知子窗口）
    └── WindowHeaderBar + notification-body
```

### 5.2 响应式

PC 端主窗口尺寸固定为 800×600，因此当前无复杂响应式需求。但抽屉宽度 80% 会在不同 DPI/缩放场景下变化。

**建议改进**：
- 抽屉建议设置 `min-width: 560px` 与 `max-width: 900px`。
- 在高 DPI 屏幕上，建议根据 `window.devicePixelRatio` 微调字体与按钮尺寸。

## 6. 交互与动效

### 6.1 通用过渡

| 元素 | 时长 | 属性 | 用途 |
|---|---|---|---|
| 标题栏按钮 hover | 150ms | background/color | 悬停反馈 |
| 关闭按钮 hover | 150ms | background/color | 危险反馈 |
| Element Plus 组件 | 默认 | — | 按钮、输入框、抽屉内置过渡 |

### 6.2 窗口控制

- 固定：切换 pinned 状态，图标高亮。
- 收起：最小化到任务栏/托盘。
- 全屏：最大化窗口。
- 关闭：隐藏到托盘（不退出应用）。

**建议改进**：
- ✅ 关闭按钮已增加 `el-tooltip` 提示“隐藏到托盘”。

### 6.3 加载状态

- iframe 加载使用 Element Plus `v-loading` 指令，显示“正在加载业务页面”。
- 调试页各检测按钮使用 `:loading` 状态。

**建议改进**：
- ✅ iframe 加载已增加 15 秒超时检测，超时后显示错误提示与“重新加载”按钮。

## 7. 可访问性（Accessibility）

### 7.1 当前优点

- 标题栏控制按钮均有 `aria-label` 与 `title` ✅。
- iframe 有 `@load` 事件处理 ✅。
- 使用 Element Plus 组件，自带键盘导航与焦点管理 ✅。

### 7.2 待改进项

| 问题 | 影响 | 建议 |
|---|---|---|
| 标题栏 logo `<img alt="应用图标">` | 已提供描述文本 | ✅ |
| 调试按钮缺少 aria-label | 仅显示“调试”文字，已可读 | 当前可接受 |
| 日志清空无确认 | 可能误删 | 增加二次确认 |
| 未处理 `prefers-reduced-motion` | 动效可能引发不适 | ✅ 已在 `App.vue` 添加全局媒体查询，禁用动画与过渡 |

## 8. 与 hub-platform 的风格统一方案

hub-platform 是运行在 client 主窗口 iframe 内的业务页面，两者视觉必须自然过渡。当前两者已较为接近，但仍存在差异。

### 8.1 当前差异对比

| 维度 | hub-platform | client-ui | 统一建议 |
|---|---|---|---|
| **定位** | 平板/Web 端业务页 | PC 桌面端基座 | 整体设计语言一致，密度按平台微调 |
| **主色** | `#2354f4` | Element Plus 默认 `#2354f4` | ✅ 已一致 |
| **页面背景** | `#f4f6f8` | `#f4f6f8` | ✅ 已一致 |
| **面板背景** | `#ffffff` | `#ffffff` | ✅ 已一致 |
| **主文本** | `#1f2937` | `#1f2937` | ✅ 已一致 |
| **次要文本** | `#4b5563` | `#4b5563` | ✅ 已一致 |
| **弱化文本** | `#6b7280` | `#6b7280` | ✅ 已一致 |
| **边框** | `#e5e7eb` | `#e5e7eb` | ✅ 已一致 |
| **danger** | `#dc2626` | `#dc2626` | ✅ 已一致 |
| **圆角 sm** | 6px | 6px | ✅ 已一致 |
| **圆角 md** | 10px | 10px | ✅ 已一致 |
| **圆角 lg** | 16px | 16px | ✅ 已一致 |
| **间距 xs** | 6px | 4px | 档位对齐，按平台密度复写 |
| **间距 sm** | 10px | 8px | 档位对齐，按平台密度复写 |
| **间距 md** | 16px | 12px | 档位对齐，按平台密度复写 |
| **间距 lg** | 20px | 16px | 档位对齐，按平台密度复写 |
| **间距 xl** | 24px | 20px | 档位对齐，按平台密度复写 |
| **阴影** | sm/md/lg | sm/md/lg | ✅ 已一致 |
| **字体** | Inter + 系统中文字体 | Inter + 系统中文字体 | ✅ 已一致 |
| **按钮高度** | 约 38px | Element Plus 默认 | 默认接近，可接受 |
| **标题栏** | 无（在 client 中） | 44px 自绘 | hub-platform 不应再显示系统/浏览器标题栏 |

### 8.2 统一后的设计令牌建议

建议将两端设计令牌合并为一套分层变量，按平台复写密度。

```css
/* 核心令牌（两端共用） */
--cpms-primary: #2354f4;
--cpms-primary-hover: #1d44c9;
--cpms-primary-dark: #0f4c9a;
--cpms-primary-light: #eef3ff;
--cpms-danger: #dc2626;
--cpms-danger-hover: #b91c1c;
--cpms-danger-light: #fef2f2;
--cpms-success: #047857;
--cpms-success-light: #d1fae5;

--cpms-text: #1f2937;
--cpms-text-secondary: #4b5563;
--cpms-text-muted: #6b7280;
--cpms-text-placeholder: #9ca3af;

--cpms-bg: #f4f6f8;
--cpms-surface: #ffffff;
--cpms-hover: #f1f4f8;
--cpms-border: #e5e7eb;
--cpms-border-light: #f1f5f9;

--cpms-radius-sm: 6px;
--cpms-radius-md: 10px;
--cpms-radius-lg: 16px;

--cpms-shadow-sm: 0 2px 8px rgba(15, 23, 42, 0.04);
--cpms-shadow-md: 0 4px 16px rgba(15, 23, 42, 0.06);
--cpms-shadow-lg: 0 8px 24px rgba(15, 23, 42, 0.1);

/* PC 端密度（client-ui） */
--cpms-space-xs: 4px;
--cpms-space-sm: 8px;
--cpms-space-md: 12px;
--cpms-space-lg: 16px;
--cpms-space-xl: 20px;

/* 平板端密度（hub-platform） */
--cpms-space-xs: 6px;
--cpms-space-sm: 10px;
--cpms-space-md: 16px;
--cpms-space-lg: 20px;
--cpms-space-xl: 24px;
```

### 8.3 统一原则

1. **颜色完全一致**：背景、文本、边框、语义色使用同一套 hex。
2. **圆角一致**：小 6px、中 10px、大 16px，两端组件统一使用。
3. **阴影一致**：client 引入 hub-platform 的三级阴影，用于抽屉、通知、悬浮按钮。
4. **间距按平台分层**：PC 端更紧凑，平板端更宽松，但档位对齐（xs/sm/md/lg/xl）。
5. **字体一致**：统一字体栈，确保 iframe 内外文字渲染一致。
6. **组件风格一致**：
   - hub-platform 的 `.button` 与 Element Plus `el-button` 视觉上应统一。
   - hub-platform 的 `.input`、`.select` 与 `el-input`、`el-select` 的 focus ring、圆角、高度应统一。
   - 状态标签（status-tag）建议 hub-platform 使用 Element Plus `el-tag` 风格或自定义但颜色与 client 一致。

### 8.4 具体统一任务清单

#### 高优先级

1. ✅ **统一背景色**：两端页面背景均为 `#f4f6f8`。
2. ✅ **统一弱化文本色**：两端均为 `#6b7280`。
3. ✅ **统一边框色**：两端均为 `#e5e7eb`。
4. ✅ **统一 danger 色**：两端均为 `#dc2626` / `#b91c1c` 组合。
5. ✅ **client 引入阴影令牌**：`--cpms-shadow-sm/md/lg`。
6. ✅ **client 引入中大圆角**：`--cpms-radius-medium: 10px`、`--cpms-radius-large: 16px`。

#### 中优先级

7. ✅ **client 扩展 spacing 档位**：已增加 `large` (16px) / `xlarge` (20px)。
8. ✅ **统一字体栈**：两端统一为 `Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif`。
9. **hub-platform 按钮与 Element Plus 按钮对齐**：高度、圆角、padding 可进一步对齐（当前 EP 默认接近）。
10. ✅ **通知子窗口增加类型图标与阴影**。

#### 低优先级

11. hub-platform 逐步将 PNG 图标替换为 SVG，与 client 的 SVG 标题栏图标风格一致。
12. 两端统一状态标签颜色映射（success/warning/error/info）。
13. 两端统一 empty / loading / failure 状态组件的视觉风格。

## 9. 页面级规范

### 9.1 主窗口（`HomeView`）

- **标题栏**：左侧 Logo + “CPMS Client”，右侧窗口控制。
- **内容区**：全屏 iframe，加载 hub-platform。
- **调试入口**：右下角悬浮按钮。
- **建议**：调试按钮在 release 模式下应隐藏或缩小为图标按钮。

### 9.2 调试抽屉

- **标题栏**：复用 `WindowHeaderBar`，仅显示关闭按钮。
- **页签**：能力检测 / 客户端日志。
- **内容区**：占满抽屉宽度，卡片式模块布局。
- **建议**：✅ 抽屉打开时覆盖半透明遮罩并禁用 iframe 交互，点击遮罩可关闭抽屉。

### 9.3 通知窗口（`NotificationView`）

- **标题栏**：通知标题 + 关闭按钮。
- **正文**：预格式化文本，自动换行。
- **建议**：已根据通知类型显示左侧色条、类型图标与标签。

## 10. 图标与图像

- 标题栏控制按钮使用内联 SVG（14px）。
- Logo 使用 `tauri.svg`（18px）。
- 业务图标主要来自 Element Plus。

**建议改进**：
- 建议建立统一 SVG 图标库（如 Lucide），与 hub-platform 未来替换的图标风格一致。

## 11. 待改进清单（按优先级）

### 高优先级

1. ✅ **统一两端颜色令牌**（背景、弱化文本、边框、danger）。
2. ✅ **client 引入阴影与中/大圆角令牌**。
3. ✅ **通知窗口增加类型图标与阴影**。
4. ✅ **日志清空增加二次确认**。
5. ✅ **iframe 加载失败增加错误提示与重试按钮**。

### 中优先级

6. ✅ **扩展 client spacing 档位**。
7. ✅ **统一字体栈**。
8. ✅ **调试抽屉增加 min/max-width 限制**。
9. ✅ **标题栏 logo 已补充 `alt="应用图标"`**。
10. ✅ **日志区撑满抽屉剩余空间**。

### 低优先级

11. 建立统一 SVG 图标库。
12. 调试模块支持折叠/展开。
13. ✅ 日志条目按级别着色（error/warn/info/debug）。
14. 支持 `prefers-reduced-motion`。

## 12. 设计原则速查

- **颜色**：以 `--cpms-*` 变量为准，禁止组件内写裸色值。
- **字体**：Inter + 系统字体回退，正文 14px，辅助 13px。
- **间距**：PC 端紧凑（4/8/12/16/20），按平台复写。
- **圆角**：小 6px、中 10px、大 16px。
- **阴影**：统一三级阴影，用于抽屉/通知/悬浮元素。
- **交互**：150ms 过渡，hover 反馈，关闭按钮危险态。
- **可访问性**：图标按钮带 aria-label，表单使用 Element Plus 原生支持。
- **跨端一致**：client 作为容器，hub-platform 作为内容，两者颜色/圆角/阴影/字体必须无缝衔接。

---

*本文档基于 2026-06-17 的代码状态生成，后续 UI 迭代请同步更新，并优先保持与 hub-platform 的跨端一致性。*
