#!/usr/bin/env node
/**
 * CPMS Client 跨平台打包脚本
 *
 * 根据当前宿主平台自动过滤可用的 bundle 格式，确保单包 / 平台包 / 全量命令
 * 都能在对应平台上一次性执行成功，不会因混入不支持的跨平台格式而报错。
 *
 * 用法：node scripts/build.js <type>
 *   type: win | linux | all | msi | nsis | deb | rpm | appimage
 */

import { execSync } from "node:child_process";
import process from "node:process";

const platform = process.platform;

// 当前宿主平台支持的 bundle 格式（与 Tauri bundler 保持一致）
const platformBundles = {
  win32: ["msi", "nsis"],
  linux: ["deb", "rpm", "appimage"],
  darwin: ["app", "dmg"],
};

// 命令类型到 bundle 格式的映射
const commandMap = {
  win: ["msi", "nsis"],
  linux: ["deb", "rpm", "appimage"],
  all: platformBundles[platform] || [],
  msi: ["msi"],
  nsis: ["nsis"],
  deb: ["deb"],
  rpm: ["rpm"],
  appimage: ["appimage"],
};

const type = process.argv[2] || "all";
const targetBundles = commandMap[type];

if (!targetBundles || targetBundles.length === 0) {
  console.error(`❌ 当前平台 "${platform}" 不支持构建类型 "${type}"`);
  console.error(`   当前平台可用格式: ${(platformBundles[platform] || []).join(", ") || "无"}`);
  process.exit(1);
}

// 过滤出当前平台真正支持的格式（防御性校验）
const availableBundles = platformBundles[platform] || [];
const filteredBundles = targetBundles.filter((b) => availableBundles.includes(b));

if (filteredBundles.length === 0) {
  console.error(`❌ 构建类型 "${type}" 在当前平台 "${platform}" 上无可用格式`);
  process.exit(1);
}

const bundlesArg = filteredBundles.join(" ");
const cmd = `pnpm tauri build --bundles ${bundlesArg}`;

console.log(`🚀 正在构建: ${bundlesArg}（平台: ${platform}）`);
console.log(`> ${cmd}\n`);

try {
  execSync(cmd, { stdio: "inherit", cwd: process.cwd() });
} catch (e) {
  process.exit(e.status || 1);
}
