# CPMS Client 设计规范

> 基于 `ui-ux-pro-max` Skill 制定的统一设计系统：Enterprise Gateway / Trust & Authority。
> 本文档描述 CPMS Client（PC 桌面端）的视觉与交互规范，并与 `cpmsClient/hub-platform` 保持跨端一致。

## 1. 项目概述

- **产品类型**：企业级打印客户端基座（PC 桌面端）。
- **技术栈**：Vue 3 + TypeScript + Vite + Element Plus + UnoCSS，Tauri 1/2 作为桌面壳。
- **目标平台**：Windows / macOS / Linux（含国产 Linux legacy）。
- **窗口形态**：
  - 主窗口：默认 800×600，无系统标题栏（`decorations: false`），背景透明（`transparent: true`），自绘 `WindowHeaderBar`。
  - 通知子窗口：400×400，屏幕右下角，背景透明（`transparent: true`），自绘标题栏。
  - 调试抽屉：从主窗口右侧滑出，宽度 80%（min 560px / max 900px）。
- **设计语言**：权威、专业、清晰、可信赖。窗口外壳采用深色标题栏（`--cpms-color-foreground`），内容区保持明亮简洁；以海军蓝（Navy）为主色、青蓝（Cyan-Blue）为强调色。

## 2. 设计语言与风格

### 2.1 整体风格

- **风格关键词**：权威、专业、紧凑、工具型、无边框窗口。
- **视觉层次**：通过自绘标题栏、面板背景色、Element Plus 组件层级区分。
- **密度**：高密度，适合 PC 端鼠标操作与信息密度较大的调试/日志场景。
- **色彩倾向**：主色为权威海军蓝 `#1E3A8A`，强调/CTA 为青蓝 `#0369A1`，危险态为红色 `#DC2626`。

### 2.2 与 ui-ux-pro-max 的契合点

| Skill 规则 | 当前实现 | 符合度 |
|---|---|---|
| `style-match`：产品类型匹配 | 企业工具型桌面客户端，Enterprise Gateway | ✅ 高 |
| `consistency`：全站风格一致 | 全部窗口共用 `tokens.css` | ✅ 高 |
| `no-emoji-icons`：不使用 emoji | 使用 SVG + Element Plus 图标 | ✅ 高 |
| `system-controls`：优先系统/成熟组件 | 使用 Element Plus | ✅ 高 |
| `elevation-consistent`：阴影统一 | 五级阴影基于统一前景色 | ✅ 高 |
| `color-accessible-pairs`：文本对比度 | 关键组合均 ≥4.5:1 | ✅ 高 |

## 3. 设计令牌（Design Tokens）

所有令牌集中在 `client-ui/src/assets/styles/tokens.css` 的 `:root` 中。

### 3.1 颜色系统

```css
/* 文本 */
--cpms-color-text-primary: #020617;
--cpms-color-text-secondary: #334155;
--cpms-color-text-muted: #64748b;
--cpms-color-text-disabled: #94a3b8;
--cpms-color-text-placeholder: #94a3b8;

/* 背景 */
--cpms-color-bg-app: #f8fafc;
--cpms-color-bg-panel: #ffffff;
--cpms-color-bg-hover: #f1f5f9;
--cpms-color-bg-active: #e2e8f0;
--cpms-color-bg-code: #f1f5f9;

/* 边框 */
--cpms-color-border: #e2e8f0;
--cpms-color-border-strong: #cbd5e1;

/* 前景/品牌表面 */
--cpms-color-foreground: #0f172a;

/* 主色 */
--cpms-color-primary: #1e3a8a;
--cpms-color-primary-hover: #1e40af;
--cpms-color-primary-active: #172554;
--cpms-color-primary-bg: #eff6ff;
--cpms-color-primary-bg-subtle: #dbeafe;
--cpms-color-primary-border: #bfdbfe;
--cpms-color-primary-text: #1e3a8a;

/* 强调色/CTA */
--cpms-color-accent: #0369a1;
--cpms-color-accent-hover: #075985;
--cpms-color-accent-bg: #f0f9ff;
--cpms-color-accent-bg-subtle: #e0f2fe;
--cpms-color-accent-border: #bae6fd;
--cpms-color-accent-text: #0369a1;

/* 语义色 */
--cpms-color-success: #047857;
--cpms-color-warning: #b45309;
--cpms-color-danger: #dc2626;
--cpms-color-danger-hover: #b91c1c;
--cpms-color-info: #0369a1;
```

**对比度检查（基于 WCAG AA）**：
- `#020617` on `#ffffff`：约 15.8:1 ✅
- `#334155` on `#ffffff`：约 9.7:1 ✅
- `#64748b` on `#ffffff`：约 5.7:1 ✅
- `#1e3a8a` on `#ffffff`：约 9.6:1 ✅
- `#0369a1` on `#ffffff`：约 5.0:1 ✅
- `#dc2626` on `#ffffff`：约 5.7:1 ✅

### 3.2 字体系统

- **字体族**：`'Plus Jakarta Sans', Inter, "Segoe UI", "PingFang SC", "Microsoft YaHei", Arial, sans-serif`
- **本地字体文件**：`client-ui/src/assets/fonts/PlusJakartaSans-{400,500,600,700}.ttf`
- **基础字号**：`16px`
- **行高**：正文 `1.5`

| 用途 | 字号 | 行高 | 字重 | 颜色 |
|---|---|---|---|---|
| 窗口标题栏文字 | 14px | 1.4 | 600 | `--cpms-color-text-primary` |
| 正文/标签 | 16px | 1.5 | 400 | `--cpms-color-text-primary` |
| 辅助说明 | 13px | 1.5 | 400 | `--cpms-color-text-secondary` |
| 代码/日志 | 13px | 1.5 | 400 | `--cpms-color-text-primary` |
| 区块标题 | 18px | 1.25 | 600 | `--cpms-color-text-primary` |
| 页面标题 | 20px | 1.25 | 700 | `--cpms-color-text-primary` |

### 3.3 间距系统

采用 4pt 网格：

```css
--cpms-space-1: 4px;
--cpms-space-2: 8px;
--cpms-space-3: 12px;
--cpms-space-4: 16px;
--cpms-space-5: 20px;
--cpms-space-6: 24px;
--cpms-space-8: 32px;
--cpms-space-10: 40px;
--cpms-space-12: 48px;
```

### 3.4 圆角系统

```css
--cpms-radius-xs: 4px;
--cpms-radius-small: 6px;
--cpms-radius-panel: 10px;
--cpms-radius-medium: 12px;
--cpms-radius-large: 16px;
--cpms-radius-xl: 20px;
--cpms-radius-full: 9999px;
```

| 用途 | 圆角 |
|---|---|
| 按钮/输入框/小标签/标题栏按钮 | `small` (6px) |
| 卡片/面板/警告条 | `panel` (10px) |
| 抽屉/大面板左侧圆角 | `large` (16px) |

### 3.5 阴影系统

基于前景色 `#020617`：

```css
--cpms-shadow-xs: 0 1px 2px rgba(2, 6, 23, 0.04);
--cpms-shadow-sm: 0 2px 8px rgba(2, 6, 23, 0.05);
--cpms-shadow-md: 0 4px 16px rgba(2, 6, 23, 0.08);
--cpms-shadow-lg: 0 8px 24px rgba(2, 6, 23, 0.12);
--cpms-shadow-xl: 0 12px 32px rgba(2, 6, 23, 0.16);
```

| 用途 | 阴影 |
|---|---|
| 卡片/代码块 | `sm` |
| 通知子窗口 | `md` |
| 调试抽屉 | `lg` |

### 3.6 动效

```css
--cpms-duration-fast: 150ms;
--cpms-duration-base: 200ms;
--cpms-duration-slow: 300ms;
--cpms-easing-base: cubic-bezier(0.4, 0, 0.2, 1);
--cpms-easing-out: cubic-bezier(0, 0, 0.2, 1);
--cpms-easing-in: cubic-bezier(0.4, 0, 1, 1);
```

| 元素 | 时长 | 属性 | 用途 |
|---|---|---|---|
| 按钮 hover/active | 150ms | background/transform | 状态反馈 |
| 卡片 hover lift | 200ms | transform/box-shadow | 悬停提升 |
| 抽屉/弹窗 | 200-300ms | transform/opacity | 进入/退出 |

## 4. 组件规范

### 4.0 UnoCSS 与 Element Plus 应用约定

- **Element Plus 是交互组件基座**：按钮、抽屉、页签、提示、空状态、确认框、loading 指令等优先使用 Element Plus。
- **Element Plus 样式通过令牌对齐**：`tokens.css` 用 `:root:root` 覆盖关键 `--el-*` 变量，使 EP 组件与 `--cpms-*` 视觉令牌一致。
- **UnoCSS 用于原子化辅助样式**：适合补充布局、间距、状态类和轻量工具类，不替代设计令牌。
- **样式优先级**：跨组件视觉规范写入 `tokens.css`；单组件结构样式写在对应 `.vue` 的 scoped CSS；局部一次性布局可用 UnoCSS。
- **图标使用**：结构图标使用 SVG（Solar / Element Plus / Lucide），不得使用 emoji。

### 4.1 窗口标题栏（`WindowHeaderBar`）

```text
┌────────────────────────────────────────┐
│  [Logo] 标题                    ─ □ ✕ │
└────────────────────────────────────────┘
```

- **高度**：`var(--cpms-headerbar-height)` = 44px。
- **背景**：`--cpms-color-foreground`（深色海军蓝 `#0F172A`），底部 1px 半透明边框。
- **标题**：左侧 Logo（20px）+ 白色文字，可拖拽区（`data-tauri-drag-region`）。
- **控制按钮**：最小 `44×44px`，图标 18px，hover 在半透明白色背景上反馈。
  - 固定/取消固定：pin 图标，`is-active` 时高亮边框色。
  - 收起：下划线图标。
  - 全屏/退出全屏：全屏图标。
  - 关闭：× 图标，hover 使用危险红底白字。
- **可访问性**：所有图标按钮带 `aria-label` 与 `title`；关闭按钮 tooltip 提示“隐藏到托盘”。

### 4.2 主窗口布局（`HomeView`）

- 标题栏 → iframe 业务页 → 右下角调试入口。
- 入口页（`EntryView`）在未设置 iframe URL 时显示；设置后切换为全屏 iframe。
- 调试抽屉宽度 `80%`，`min-width: 560px`，`max-width: 900px`。

### 4.3 入口页（`EntryView`）

- 居中卡片式布局，最大宽度 520px，带柔和投影与细边框。
- 品牌区：纯色主色圆角图标（64px）+ 大标题 + 辅助说明。
- 表单输入框：48px 高，左侧链式图标前缀，带聚焦光环；错误态显示在字段下方。
- 提交按钮：纯色主色、48px 高、全宽，hover 加深，无渐变。
- 最近地址：胶囊标签 + 44×44 操作按钮。
- 服务器地址输入支持 `Enter` 提交。

### 4.4 调试能力检测页（`ExampleView`）

- 单列卡片流，每个 `section.card` 一个能力模块。
- 卡片：背景 `--cpms-color-bg-panel`、边框 `--cpms-color-border`、圆角 `panel`、padding `16px`。
- 结果展示：`<pre class="result">`，背景 `--cpms-color-bg-code`，支持一键复制。
- 各模块支持折叠/展开。

### 4.5 客户端日志页（`LogView`）

- 工具栏：类别下拉 + 刷新/复制/清空按钮。
- 日志文本区撑满抽屉剩余空间，按级别着色（error/warn/info/debug）。
- 清空操作需二次确认。

### 4.6 通知子窗口（`NotificationView`）

- 尺寸 400×400，卡片式窗口，窗口背景透明，卡片本身使用 `--cpms-color-bg-panel` 保持可读性。
- 正文顶部显示类型图标 + 类型名称（info / success / warning / error），下方为消息内容。
- 关闭按钮最小 `44×44px`。

### 4.7 错误提示（`ErrorNotice`）

- 使用 `el-alert`，类型 error/warning。
- 显示标题 + 来源/错误码描述，可关闭。

## 5. 布局规范

### 5.1 页面骨架

```text
App.vue
├── HomeView（主窗口）
│   ├── WindowHeaderBar
│   ├── main.iframe-root
│   │   ├── EntryView
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

PC 端主窗口尺寸固定为 800×600，主要响应式场景为抽屉宽度与高 DPI 缩放：
- 抽屉 `min-width: 560px`，`max-width: 900px`。
- 所有固定高度组件需按 `rem` / 令牌取值，避免字号提升后截断。

## 6. 交互与动效

- 按钮 hover/active 在 150ms 内完成。
- 卡片 hover 仅使用 `transform: translateY(-2px)` + `box-shadow`，时长 200ms。
- 支持 `prefers-reduced-motion: reduce`：全局禁用动画与过渡。
- 所有可点击元素最小 `44×44px`（PC 鼠标操作可适当压缩视觉尺寸，但需保证 hit area）。

## 7. 可访问性（Accessibility）

- 图标按钮均带 `aria-label` 与 `title`。
- 表单输入使用可见 `label`。
- 错误信息使用 `aria-describedby` 与输入框关联。
- 焦点环：`outline: 2px solid var(--cpms-color-primary)`，`outline-offset: 2px`。
- 颜色不作为唯一信息载体，状态/错误需配合图标或文本。
- 支持 `prefers-reduced-motion`。

## 8. 与 hub-platform 的风格统一

hub-platform 运行在 client 主窗口 iframe 内，两者视觉必须自然过渡。

| 维度 | 统一值 |
|---|---|
| 主色 | `#1E3A8A` |
| 强调色 | `#0369A1` |
| 页面背景 | `#F8FAFC` |
| 面板背景 | `#FFFFFF` |
| 主文本 | `#020617` |
| 次要文本 | `#334155` |
| 弱化文本 | `#64748B` |
| 边框 | `#E2E8F0` |
| 字体 | Plus Jakarta Sans |
| 阴影 | 基于 `#020617` 的五级阴影 |

client 负责窗口外壳与容器，hub-platform 负责业务内容；两者共享语义色板，仅在间距密度上按平台微调。

## 9. 页面级规范

### 9.1 主窗口（`HomeView`）

- 标题栏左侧 Logo + “CPMS Client”。
- 内容区显示入口页或 iframe。
- 调试入口为右下角悬浮按钮。

### 9.2 调试抽屉

- 标题栏复用 `WindowHeaderBar`，仅显示关闭按钮。
- 页签：能力检测 / 客户端日志。
- 内容区卡片式模块布局。

### 9.3 通知窗口（`NotificationView`）

- 标题栏显示通知标题 + 关闭按钮。
- 正文预格式化文本，自动换行，按类型显示色条。

## 10. 图标与图像

- 标题栏控制按钮使用内联 SVG（14px）。
- Logo 使用 `tauri.svg`（18px），带 `alt` 文本。
- 业务图标优先使用 Element Plus / Solar / Lucide SVG。

## 11. 待改进清单

- [ ] 标题栏控制按钮扩展到 `44×44px` hit area。
- [ ] 全部页面按新颜色、字体、动效令牌调整。
- [ ] 增加 `prefers-reduced-motion` 降级验证。
- [ ] 验证与 hub-platform 的 iframe 内外视觉一致性。

## 12. 设计原则速查

- **颜色**：以 `--cpms-*` 变量为准，禁止组件内写裸色值。
- **字体**：Plus Jakarta Sans + 系统回退，正文 16px，行高 1.5。
- **间距**：4pt 网格，PC 端保持紧凑。
- **圆角**：小 6px、中 10px、大 16px。
- **阴影**：统一五级阴影，基于 `#020617`。
- **交互**：150–300ms 过渡，hover 反馈，关闭按钮危险态。
- **可访问性**：图标按钮带 aria-label，表单使用可见 label，焦点环可见。
- **跨端一致**：client 作为容器，hub-platform 作为内容，颜色/圆角/阴影/字体无缝衔接。

---

*本文档基于 2026-06-22 的统一设计系统制定，后续 UI 迭代请同步更新。*
