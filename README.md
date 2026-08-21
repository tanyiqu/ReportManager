# ReportManager

一个本地优先的个人工作记录桌面应用，用于集中整理日报、周报和每日例会记录。项目采用 Tauri 桌面壳与 React 前端，运行时数据保存在本机 SQLite 数据库中，不依赖账号或网络服务。

> 当前处于原型开发阶段：界面、SQLite 初始化、系统托盘和单实例窗口行为已实现；记录的持久化增删改查、搜索、Markdown 导出以及周报、例会的完整业务流程尚未实现。

## 项目介绍

ReportManager 面向需要持续沉淀工作进展的个人用户，提供桌面端的工作记录入口。应用默认使用简体中文，并将发布版的可变数据存放在可执行文件同级的 `data` 目录，便于备份和整体迁移。

## 功能特性

当前代码已实现：

- 简体中文桌面端布局：首页、日报、周报、例会记录和设置导航。
- 今日日报编辑界面，包含标题、自由正文及完成事项、进行中事项、问题与风险、下一步计划、标签等结构化字段入口。
- 本地 SQLite 数据库初始化，并创建 `records` 表及查询索引。
- 发布版使用可执行文件同级 `data/report-manager.db`；开发版使用 `src-tauri/data/report-manager.db`。
- 主窗口在 React 首帧挂载后显示，减少启动时的空白窗口。
- 关闭主窗口时隐藏到系统托盘；托盘菜单可显示窗口或退出程序。
- 单实例启动：再次启动程序时恢复已有主窗口。
- 使用根目录 `logo.png` 生成的 Tauri 图标配置用于应用和系统托盘。
- 左侧菜单顶部提供完整显示的 ReportManager 工具栏，包含侧栏展开/收起与搜索入口；收缩后仅保留展开按钮，状态会保存到本地数据库。
- 左侧导航与右侧内容区使用独立滚动容器，滚动工作区不会带动侧栏。
- 设置项采用即时保存，并在右上角以可点击关闭、可纵向排队的 Toast 提示保存结果；提示从右侧滑入并渐变消失。
- 设置页可设置应用启动后的默认页面，并持久化周起始日、导出目录等偏好。
- 设置页使用单一“菜单管理”入口：可在对话框内拖拽自定义菜单并即时保存排序，也可分别重命名、修改 SVG 图标、删除或添加菜单；首页始终置顶、设置始终置底，且两者不可拖拽或删除。
- 自定义菜单仅在没有关联报告时可删除；菜单配置和报告的 `menu_id` 已纳入 SQLite 迁移，便于后续记录持久化接入。
- 菜单与偏好设置保存已避免 SQLite 互斥锁重入：收缩侧栏、切换收缩后的菜单、设置启动页及菜单增删/排序不会再造成界面未响应。
- 菜单 SVG 图标使用带 SVG XML 命名空间的 base64 数据 URI 作为 CSS 掩码来源；渲染时会自动兼容数据库中缺少 `xmlns` 的旧图标，展开和收缩状态下均可显示。

尚未完成的能力包括记录的实际保存、读取、删除和搜索，纯文本复制、Markdown 导出，以及周报和例会记录的完整编辑流程。界面中的对应按钮或提示目前不代表功能已可用。

## 技术栈

- [Tauri 2](https://v2.tauri.app/)：跨平台桌面应用框架。
- Rust 2021：Tauri 后端、系统托盘、单实例管理和 SQLite 访问层。
- React 19 + TypeScript：前端界面。
- Vite 8：前端开发服务器与构建工具。
- SQLite（`rusqlite`，启用 `bundled`）：本地数据存储。
- pnpm：JavaScript 依赖管理；仓库包含 `pnpm-lock.yaml`。

## 环境要求

项目未在仓库中固定 Node.js、pnpm 或 Rust 的最低版本。开发前请安装以下工具，并满足 Tauri 2 对当前操作系统的系统依赖要求：

| 工具 | 用途 | 版本说明 |
| --- | --- | --- |
| Node.js | 运行 Vite 与 Tauri CLI | 未在项目配置中声明最低版本 |
| pnpm | 安装和执行前端依赖 | 未在项目配置中声明最低版本 |
| Rust（stable）及 Cargo | 编译 Tauri 后端 | `Cargo.toml` 使用 Rust 2021 edition，未固定工具链版本 |
| Tauri 2 平台依赖 | 构建桌面 WebView 与安装包 | 请按目标操作系统配置 Tauri 2 官方前置依赖 |

项目优先面向 Windows 10/11；Tauri 配置保留了跨平台图标资源。Windows 开发和运行环境还应具备可用的 WebView2 Runtime。

## 安装依赖

在项目根目录执行：

```bash
pnpm install
```

该命令会依据 `pnpm-lock.yaml` 安装前端依赖。Rust 依赖由 Cargo 在首次执行 Tauri 开发或构建命令时解析和编译。

## 开发运行

### 前端开发

仅启动 Vite 前端开发服务器：

```bash
pnpm dev
```

服务固定运行在 `http://localhost:1420`；端口被占用时会直接失败。

### Tauri 开发运行

启动前端开发服务器并打开原生桌面窗口：

```bash
pnpm tauri dev
```

`src-tauri/tauri.conf.json` 中的 `beforeDevCommand` 已配置为 `pnpm dev`，通常无需手动并行启动前端服务。

## 项目构建

### 前端构建

执行 TypeScript 类型检查并生成前端产物：

```bash
pnpm build
```

构建结果输出至 `dist/`。

### Tauri Build

编译 Rust 后端、构建前端并执行 Tauri 打包：

```bash
pnpm tauri build
```

该命令会执行配置中的 `beforeBuildCommand`（即 `pnpm build`）。常规 Tauri 构建产物位于 `src-tauri/target/release/`，安装包位于其 `bundle/` 子目录。

### Windows 打包

在 Windows 环境执行同一构建命令即可。当前 `bundle.targets` 配置为 `"all"`，由 Tauri 为当前平台生成其支持的全部打包目标：

```powershell
pnpm tauri build
```

生成文件的具体扩展名和目录由安装的 Tauri 工具链及平台决定，请以 `src-tauri/target/release/bundle/` 中的实际结果为准。

### 运行发布版本

安装或直接运行构建产物中的应用可执行文件。首次运行会在该可执行文件同级创建 `data/` 目录，其中包含 `report-manager.db`。请将应用置于当前用户具有写入权限的位置；复制应用目录及其 `data/` 目录即可迁移本地数据。

## 项目结构

```text
ReportManager/
├── src/                         # React 前端源码
│   ├── App.tsx                  # 页面布局、日报编辑原型与窗口显示调用
│   ├── App.css                  # 前端样式
│   └── main.tsx                 # React 挂载入口
├── public/                      # 前端静态资源
├── src-tauri/                   # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs              # Rust 可执行程序入口
│   │   ├── lib.rs               # Tauri 初始化、命令、托盘与窗口行为
│   │   ├── database.rs          # SQLite 连接与数据库初始化
│   │   └── models.rs            # 记录数据模型
│   ├── migrations/
│   │   ├── 0001_initial.sql     # records 表及索引的初始化 SQL
│   │   └── 0002_navigation_preferences.sql # 菜单、偏好设置及记录菜单关联
│   ├── capabilities/            # Tauri 权限能力配置
│   ├── icons/                   # 各平台应用图标资源
│   ├── Cargo.toml               # Rust 依赖与构建配置
│   └── tauri.conf.json          # Tauri 窗口、构建与打包配置
├── logo.png                     # 应用图标源文件
├── package.json                 # 前端依赖与 npm scripts
├── pnpm-lock.yaml               # pnpm 锁文件
├── vite.config.ts               # Vite 开发服务器配置
├── clean.bat                    # Windows 构建产物与缓存清理脚本
└── LICENSE                      # GPL-3.0 许可证文本
```

## 配置说明

当前项目没有 `.env`、`.env.example`、Dockerfile、`requirements.txt` 或 CI/CD 配置文件；无需配置环境变量即可运行。

| 配置位置 | 作用 | 当前关键配置 |
| --- | --- | --- |
| `package.json` | 前端脚本和依赖 | `dev`、`build`、`preview`、`tauri` |
| `vite.config.ts` | Vite 开发服务 | 端口 `1420`、严格端口检查、忽略 `src-tauri` 文件监听 |
| `src-tauri/tauri.conf.json` | Tauri 应用配置 | 产品名 `ReportManager`、应用标识符、窗口尺寸、前后端构建命令、打包目标 |
| `src-tauri/capabilities/default.json` | Tauri 权限 | 主窗口的默认核心权限与 opener 插件权限 |
| `src-tauri/migrations/0001_initial.sql` | 数据库结构 | `records` 表及类型、日期、更新时间索引 |
| `src-tauri/migrations/0002_navigation_preferences.sql` | 菜单与偏好设置 | 菜单 SVG、显示顺序、启动默认页、侧栏状态与 `records.menu_id` |

开发模式数据库位于 `src-tauri/data/`；发布模式数据库位于应用可执行文件同级的 `data/`。两者均被 Git 忽略，不会提交到仓库。

## 使用说明

1. 执行 `pnpm tauri dev` 启动应用。
2. 在首页点击“新建今日日报”，填写标题和正文；右侧可填写结构化字段。
3. 在左侧顶部工具栏中使用菜单按钮收起或展开侧栏；收起后仅显示该按钮，悬浮提示会变为“展开侧边栏”。搜索入口当前仅展示提示，尚未接入搜索流程。
4. 在“设置 > 菜单管理”中拖拽自定义菜单左侧手柄调整顺序；每行右侧可分别重命名、修改图标或删除。首页和设置不可移动或删除；底部按钮可添加自定义菜单。
5. 设置项会即时写入本地数据库，右上角会显示可排队的“已保存设置”提示；点击任一提示可立即关闭。
6. 点击“保存草稿”目前只会更新前端提示，不会写入数据库。
7. 关闭窗口后，应用会隐藏到系统托盘；点击托盘图标或选择“显示主窗口”可恢复窗口。选择“退出程序”才会结束应用。

周报、例会、搜索、复制和导出入口已在界面中展示，但其业务逻辑尚待接入。

## 常见问题

### `pnpm` 命令不可用

请先安装 pnpm，并重新打开终端后执行 `pnpm install`。

### `pnpm tauri dev` 无法启动

确认已安装 Rust stable、Cargo 和目标操作系统所需的 Tauri 2 前置依赖。Windows 环境还需确认 WebView2 Runtime 可用。

### 端口 1420 被占用

`vite.config.ts` 启用了 `strictPort`，不会自动切换端口。停止占用 `1420` 的进程后重新执行：

```bash
pnpm tauri dev
```

### 找不到本地数据库

开发模式请检查 `src-tauri/data/report-manager.db`；发布模式请检查应用可执行文件同级的 `data/report-manager.db`。数据库会在 Tauri 应用首次成功启动时创建。

### 左侧菜单图标不显示

菜单图标通过 CSS `mask-image` 加载独立的 SVG data URI。旧版生成的 SVG 根节点缺少 `xmlns="http://www.w3.org/2000/svg"`，部分 WebView 无法将其作为独立 SVG 图片解析，最终显示为透明掩码。当前版本会在生成新图标时写入命名空间，并在渲染时自动补全数据库中的旧图标，无需删除或重建本地数据库。

### 如何清理构建产物

在 Windows 项目根目录执行：

```powershell
.\clean.bat
```

该脚本会清理 `dist`、Rust/Tauri `target`、Vite 缓存、覆盖率目录和项目日志；不会删除 `node_modules`、源代码、配置或本地数据。

## License

本项目采用 [GNU General Public License v3.0](LICENSE) 许可证。详见 [LICENSE](LICENSE)。
