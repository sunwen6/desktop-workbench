#!/usr/bin/env bash
# 启动脚本 —— 桌面待办工作台（Tauri v2 + Rust + React）
# 用法：
#   ./start.sh              # 默认：启动完整桌面应用（首次会编译 Rust，较慢）
#   ./start.sh dev          # 同上，显式指定完整桌面应用
#   ./start.sh web          # 只启动前端 Vite 页面（浏览器访问，无桌面窗口，Rust 命令不可用）
#   ./start.sh test         # 跑前端单元测试（vitest）
#   ./start.sh build        # 构建生产版前端（tsc + vite build）
#   ./start.sh check        # 检查工具链与依赖是否就绪，不做任何改动

set -euo pipefail

# ---------- 工具链检查 ----------
check_tools() {
  local ok=1
  for cmd in node npm cargo; do
    if command -v "$cmd" >/dev/null 2>&1; then
      echo "  ✓ $cmd: $($cmd --version 2>&1 | head -1)"
    else
      echo "  ✗ 缺少 $cmd（请先安装）"
      ok=0
    fi
  done
  if ! command -v rustc >/dev/null 2>&1; then
    echo "  ✗ 缺少 rustc（Rust 工具链，桌面模式必需）"
    ok=0
  else
    echo "  ✓ rustc: $(rustc --version 2>&1 | head -1)"
  fi
  # macOS 提示
  if [[ "$(uname -s)" == "Darwin" ]] && ! xcode-select -p >/dev/null 2>&1; then
    echo "  ! 未检测到 Xcode Command Line Tools，可运行: xcode-select --install"
  fi
  return $((1 - ok))  # ok=1 -> 0 成功；ok=0 -> 1 失败
}

# ---------- 前端依赖 ----------
ensure_deps() {
  if [[ ! -d node_modules ]]; then
    echo "==> 安装前端依赖 (npm install)..."
    npm install
  else
    echo "==> 前端依赖已就绪，跳过 npm install"
  fi
}

mode="${1:-dev}"

case "$mode" in
  check)
    echo "==> 检查工具链与依赖..."
    check_tools || { echo "工具链不完整，桌面模式无法运行。"; exit 1; }
    if [[ -d node_modules ]]; then echo "  ✓ node_modules 已存在"; else echo "  ! node_modules 不存在（首次运行会自动 npm install）"; fi
    echo "检查完成。"
    ;;
  web)
    ensure_deps
    echo "==> 启动前端开发服务器（无桌面窗口）..."
    npm run dev
    ;;
  test)
    ensure_deps
    echo "==> 运行前端单元测试..."
    npm test
    ;;
  build)
    ensure_deps
    echo "==> 构建生产版前端..."
    npm run build
    ;;
  dev)
    echo "==> 检查工具链..."
    check_tools || { echo "桌面模式需要 Node + Rust 工具链。如只看 UI 可改用: ./start.sh web"; exit 1; }
    ensure_deps
    echo "==> 启动完整桌面应用 (npm run tauri dev)..."
    echo "    首次运行会编译 Rust 依赖，需要几分钟，请耐心等待。"
    npm run tauri dev
    ;;
  *)
    echo "未知模式: $mode"
    echo "用法: $0 [dev|web|test|build|check]"
    exit 2
    ;;
esac
