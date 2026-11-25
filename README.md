# 桌面待办工作台（Desktop Workbench）

一款面向 macOS 的本地桌面待办工作台：把日常事项拆成"磁贴"，支持快速捕获、编辑、待办清单，并内置本地 MCP 服务供 AI Agent 接入。

技术栈：**Tauri v2 + Rust（后端）+ React + TypeScript + Vite**。所有数据保存在本机，MCP 服务默认只监听本机回环地址（127.0.0.1）。

---

## 快速启动

前提：已安装 **Node.js**、**npm** 和 **Rust 工具链**（`cargo` / `rustc`）。

```bash
# 1) 进入项目目录
cd /Users/sunwen/workspace/code/workbench

# 2) 赋予启动脚本可执行权限（仅首次需要）
chmod +x start.sh

# 3) 启动完整桌面应用
./start.sh
```

首次运行会自动执行 `npm install` 安装前端依赖，并编译 Rust 代码（`target/` 会重新生成，需几分钟），之后自动弹出桌面窗口。改前端代码热更新；改 Rust 代码会自动重编译重启。

### 先自检工具链（可选）

不确定环境是否就绪时，可先跑检查模式，只读不做改动：

```bash
./start.sh check
```

---

## 启动脚本用法

`start.sh` 支持以下模式：

| 命令 | 作用 |
|------|------|
| `./start.sh` 或 `./start.sh dev` | 启动**完整桌面应用**（Rust 后端 + 前端 + 桌面窗口），推荐 |
| `./start.sh web` | 只启动**前端 Vite 页面**（浏览器访问，无窗口；`invoke()` 相关功能不可用） |
| `./start.sh test` | 运行前端单元测试（vitest） |
| `./start.sh build` | 构建生产版前端（`tsc` + `vite build`） |
| `./start.sh check` | 检查 Node/npm/Rust 工具链与依赖是否就绪 |

## 不用脚本的手动方式

```bash
npm install           # 安装前端依赖（首次）
npm run tauri dev     # 启动完整桌面应用
npm run dev           # 或：只启动前端开发服务器
npm test              # 跑测试
npm run build         # 构建生产版前端
```

---

## 项目结构

```
.
├── src/                 # React 前端（磁贴网格、快速捕获、待办清单等）
├── src-tauri/           # Rust 后端（命令、磁贴/待办存储、本地 MCP 服务）
│   ├── src/             #   业务实现
│   ├── tests/           #   Rust 单元测试
│   └── tauri.conf.json  #   Tauri 配置
├── start.sh             # 启动脚本（本文件开头介绍）
└── package.json         # 前端依赖与脚本
```

## 注意事项

- **构建产物不入库**：`node_modules`、`src-tauri/target`、`dist` 已在 `.gitignore` 中排除，首次运行会重新生成。
- **MCP 服务**：默认只监听 `127.0.0.1`，端口在 17321–17330 间自动选择；访问令牌保存在本机应用数据目录，权限为 600。
- **平台**：当前在 macOS 上开发验证；Windows/Linux 需按 Tauri v2 各自的系统依赖另行配置。
