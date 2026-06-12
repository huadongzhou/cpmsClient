# Legacy Linux deb 构建复盘与排错手册

> 适用对象：`.github/workflows/build-release.yml` 的 `build-linux` (v1 legacy) 矩阵。
> 时间：2026-06-12 适配过程的完整记录。后续遇到类似问题先查本文档。

## 0. 问题本质：构建环境与目标环境的"代差"

构建发生在 Ubuntu 22.04 runner，目标是麒麟 V10（juniper，约等于 Ubuntu 16.04，2016 年水平），代差 6 年，在四个层面各自爆发：

| 层面 | 典型表现 | 解法 |
| --- | --- | --- |
| C 库链接层（glibc） | 安装即报 `GLIBC_2.xx not found` | Zig 链接器锁 glibc 2.23 |
| GUI 动态符号层（GTK/webkit 生态） | 启动瞬间 `undefined symbol: xxx` | vendor 补丁 tao/wry 改调老 API + CI 符号硬门禁 |
| 网页内容层（JS 引擎） | 窗口正常但白屏 | 前端构建目标降 ES2015 + 关加速合成 |
| 供应链层（依赖获取） | CI 下载/编译第三方库失败 | 换官方 GitHub release 源 + SHA256 校验 |

目标环境画像（实机 `dpkg -l` / `ldd --version` 采集）：

| 库 | 麒麟 V10 (juniper, x86_64) | 统信 | 构建容器基准 (ubuntu:16.04) |
| --- | --- | --- | --- |
| glibc | 2.23 | 较新 | 2.23 ✓ |
| libgtk-3-0 | 3.18.9 (kord) | ≥3.22 | 3.18.9 ✓ |
| libwebkit2gtk-4.0-37 | 2.20.5 (kord) | ≥2.22 | 2.20.5 ✓ |
| libglib2.0-0 | 2.48.2 | 较新 | 2.48.2 ✓ |
| gdk-pixbuf | ~2.32 | ≥2.36.8 | 2.32.2 ✓ |

结论：**ubuntu:16.04 容器与麒麟 V10 完全同代，可作为麒麟的忠实代理**（已实测：容器 `ldd -r` 清单与真机逐项吻合）。统信库较新，所有面向麒麟的修复对统信向后兼容。

---

## 1. CI 构建环境问题（GitHub Actions / 供应链 / 编译期）

这一类错误在 CI 日志里直接可见，与目标设备无关。

### 1.1 OpenSSL 旧版 tarball 404

- 现象：`Build OpenSSL` 步骤 `curl: (22) ... error: 404`，双架构同挂。
- 根因：openssl.org 已下线旧版本源码包（`www.openssl.org/source/old/...` 全部 404）。
- 解决：改从 OpenSSL 官方 GitHub release 资产下载并校验 SHA256。
- 提交：`0e6d404`。

### 1.2 openssl-sys 拒绝 OpenSSL 1.0.2

- 现象：编译 panic：`This crate is only compatible with OpenSSL (version 1.1.0, 1.1.1, 3.x, or 4.x)`。
- 根因：现代 Rust 生态（openssl-sys ≥0.9.10x）已放弃 1.0.x；"动态链接麒麟系统 openssl 1.0.2g"这条路线不可行。
- 解决：静态嵌入 OpenSSL 1.1.1w（`no-shared no-dso -fPIC` + `OPENSSL_STATIC=1`）。麒麟（1.0.2g）与统信（无 1.0.x）都不再需要系统 openssl；TLS 1.0-1.3 与 legacy 密码套件保留。
- 提交：`89c2f8e`。

### 1.3 vendored tao 编译失败 E0658

- 现象：`error[E0658]: attributes on expressions are experimental`，指向 `vendor/tao-0.16.11/.../monitor.rs`。
- 根因：历史 tao 补丁把 `#[allow(deprecated)]` 挂在尾表达式上（语句位置合法、表达式位置非法）；该补丁此前从未在 CI 过过编译（更早的 OpenSSL 环节先挂）。
- 解决：属性移到函数级别；补丁逻辑（GTK 3.0 老 API 替代 3.22 monitor API）不变。
- 提交：`9610143`。

### 1.4 符号检查假阴性（工具自身的坑）

- 现象：新加的"麒麟同代库符号检查"首轮报零缺失，但真机明明报 undefined symbol。
- 根因：检查脚本指向 `target/release/cpms-client`，而 tauri v1 打包时已把二进制重命名为 `cpms-client-v1`；`ldd -r` 对不存在的文件无输出，被误判为通过。**"没报错" ≠ "没问题"，检查工具必须先证明跑在了真对象上。**
- 解决：按候选名探测（`cpms-client-v1` → `cpmsClient-v1` → `cpms-client`），找不到即红灯并列目录；同时升级为硬门禁。
- 提交：`3526bdd`（暴露问题）、`32a0fea`（修复路径）。

---

## 2. 麒麟目标环境问题（运行期）

这一类错误 CI 全绿但真机失败，需要真机数据（`ldd -r`、`dpkg -l`）+ CI 门禁复现。

### 2.1 启动即死：5 个 undefined symbol

- 现象：`undefined symbol: gdk_pixbuf_calculate_rowstride / gdk_display_get_default_seat / gdk_seat_get_pointer / webkit_javascript_result_get_js_value / jsc_value_to_string`。
- 机制：Rust 默认 BIND_NOW 链接，启动时一次性解析全部符号；在 22.04 上编译引用了 GTK 3.20+/webkit 2.22+ 符号，麒麟老库没有。加载器每次只报第一个缺失 → 真机排查会变成打地鼠，务必用 `ldd -r` 一次拿全。
- 解决（全部改调老 API，新系统行为不变）：

| 符号 | 需要版本 | 调用方 | 修复 |
| --- | --- | --- | --- |
| `gdk_pixbuf_calculate_rowstride` | gdk-pixbuf 2.36.8 | tao `icon.rs` | 内联 RGBA8 行距算式 |
| `gdk_display_get_default_seat` / `gdk_seat_get_pointer` | GTK 3.20 | tao `event_loop.rs`（拖拽/光标） | 改 GTK 3.0 `DeviceManager` API |
| `webkit_javascript_result_get_js_value` / `jsc_value_to_string` | webkit 2.22 | **wry IPC 消息转换**（所有 invoke 必经） | vendor wry 0.24.12，改走 JavaScriptCore C API（`JSValueToStringCopy` 一族） |

- 提交：`3b8de63`（pixbuf）、`091eb5d`（seat + jsc，引入 `vendor/wry-0.24.12` 并加入 `[patch.crates-io]`）。

### 2.2 窗口空白（符号干净之后）

- 现象：安装成功、窗口弹出、内容全白；日志仅 `Gdk-CRITICAL: gdk_window_thaw_toplevel_updates`。
- 注意：该 Gdk-CRITICAL 是 tao 在 GTK 3.18 上 freeze/thaw 计数不匹配的噪音，**不是白屏元凶**。
- 嫌疑与解法（三管齐下）：
  1. webkit 2.20 的 JS 引擎 ≈ Safari 11，ES2020 语法（`?.`、`??`）直接 SyntaxError 整页崩 → `client-tauri1/vite.config.ts` 构建目标降 `es2015`；
  2. 老 webkit 加速合成输出空白纹理（经典问题）→ `lib.rs run()` 内置 `WEBKIT_DISABLE_COMPOSITING_MODE=1`（用户环境变量可覆盖）；
  3. 开启 tauri `devtools` 特性，真机右键 → 检查元素 → Console 直接看剩余报错（稳定后应移除）。
- 提交：`13923ee`。
- 真机判别技巧：旧包直接跑 `WEBKIT_DISABLE_COMPOSITING_MODE=1 cpms-client-v1`，能区分合成问题与 JS 语法问题。

---

## 3. 统信目标环境

全程未出问题（GTK ≥3.22 / webkit ≥2.22 / gdk-pixbuf ≥2.36.8，符号都在；JS 引擎较新）。作为对照组价值：**统信成功 + 麒麟失败 = 问题必在两者库版本差异里**，按这个差异查版本引入点即可定位。

---

## 4. 排查工具箱

```bash
# 真机：一次列出全部缺失符号（替代启动打地鼠）
ldd -r /usr/bin/cpms-client-v1 2>&1 | grep -E "undefined symbol|not found"

# 真机：目标库版本画像
dpkg -l | grep -E "libwebkit2gtk|libgtk-3|gdk-pixbuf|libglib2|libsoup|appindicator"
cat /etc/os-release; uname -m; ldd --version | head -1

# 产物校验（CI 已自动做，手工复核用）
readelf -V binary | grep -o 'GLIBC_[0-9.]*' | sort -Vu     # glibc 符号版本上限
readelf -d binary | grep NEEDED                             # 动态依赖（不应有 libssl/libcrypto）
```

CI 内的长效机制（见 workflow 各步骤）：

- **Kylin-era 符号硬门禁**：ubuntu:16.04 容器 `ldd -r` 全量解析，引入过新符号即红灯——问题永远停在 CI，不再流到真机；
- GLIBC ≤2.23 断言、禁止动态 libssl/libcrypto、禁止 GTK 3.22 monitor 符号、deb 转 gzip（老 dpkg 不认 zstd）。

## 5. 维护注意事项

- `client-tauri1/src-tauri/vendor/` 下的 **tao-0.16.11 与 wry-0.24.12 是打过补丁的 fork**（经 `[patch.crates-io]` 接入）。升级 tauri v1 相关依赖时必须重新评估这两个补丁是否仍被需要/兼容。
- 升级任何前端依赖后，留意 `vite.config.ts` 的 `target: "es2015"` 仍然生效（webkit 2.20 約 Safari 11 引擎）。
- `devtools` 特性是排障期临时开启的，麒麟/统信验证稳定后应从 `Cargo.toml` 移除。
