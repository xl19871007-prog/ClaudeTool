# ClaudeTool 工作计划（执行手册）

> **本文与 ROADMAP.md 的关系**：
> - **ROADMAP.md（位于 `ClaudeTool开发文档/2.工作规范（强烈建议）/`）是「计划层」**：阶段、里程碑、当前 2 周待办、已完成、已搁置——属于活文档，频繁更新。
> - **本文 WORK_PLAN.md 是「执行层」**：把 10 周拆到周 + 任务级，每个任务有验收标准和依赖。一次性产出，主要在 MVP 周期内参考。
>
> **使用规则**：
> 1. 完成任务后**只更新 ROADMAP.md** 的"当前待办"和"已完成"两段，不要回头改 WORK_PLAN
> 2. 如果范围变了（删任务/加任务），先更 PRD 或 DECISIONS（开 ADR），再回头同步 ROADMAP，最后才动 WORK_PLAN
> 3. 任务卡颗粒度约 0.5–2 天/项
>
> **依据来源**：12 份开发文档 + REVIEW_LOG.md（4 轮评审结论）

---

## 总览

- **MVP 周期**：2026-04-20 ~ 2026-06-29（10 周）
- **里程碑**：M1–M8 + Release（共 9 个，详见 ROADMAP §3）
- **MVP 范围**：6 P0 功能（F1/F2/F4/F5/F6/F7）+ 1 P1 功能（F9）
- **暂缓**：F3（Token 监控，等官方 API）、F8（对话引导，V2）
- **核心 ADR**：见 REVIEW_LOG.md "ADR 全表"

---

## Day 1 启动 Checklist（今天就做）

按顺序执行，预计 2–3 小时：

```bash
# 1. 工作目录定位
cd D:\ClaudeTool

# 2. 创建公开 GitHub 仓库（用 gh CLI 或网页都可）
gh repo create ClaudeTool --public --description "Claude Code 的小白桌面入口" --license MIT

# 3. 在 D:\ClaudeTool\ 初始化 Git
git init
git remote add origin https://github.com/<你的用户名>/ClaudeTool.git

# 4. 写 .gitignore（关键：排除模板目录）
cat > .gitignore <<'EOF'
# Doc template (managed separately)
AI开发文档模板/

# Tauri / Rust
app/src-tauri/target/
**/*.rs.bk

# Node
app/node_modules/
app/dist/
app/.vite/

# IDE
.vscode/
.idea/
*.swp
.DS_Store

# Local
*.local
.env*
EOF

# 5. 写 README.md（最简版，后续补全）
cat > README.md <<'EOF'
# ClaudeTool

Claude Code 的小白桌面入口。详见 [开发文档](./ClaudeTool开发文档/1.核心文件（必做）/1.CLAUDE.md)。

**当前状态**：MVP 开发中（2026-04-20 启动）
**License**：MIT
EOF

# 6. 初始化 Tauri 2 项目到 app/ 子目录
pnpm create tauri-app@latest app
# 选项：React + TypeScript + Vite + pnpm

# 7. 首次提交
git add .gitignore README.md MARKET_RESEARCH.md REVIEW_LOG.md WORK_PLAN.md ClaudeTool开发文档/ app/
git commit -m "chore: bootstrap project with docs and Tauri scaffold"
git branch -M main
git push -u origin main
```

**Day 1 验收**：`https://github.com/<你>/ClaudeTool` 可访问，main 分支有提交，本地 `cd app && pnpm tauri dev` 能弹出 Tauri 默认窗口。

---

## Week 1（4/20–4/27）：M1 脚手架可启动

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 1.1 | 完成 Day 1 启动 Checklist | GitHub 仓库公开、Tauri 默认窗口可启动 | — |
| 1.2 | 加 `tauri-plugin-pty` 依赖（Cargo.toml） | `cargo build` 通过 | 1.1 |
| 1.3 | 装 shadcn/ui + Tailwind + Zustand + xterm.js + xterm-addon-fit | `pnpm install` 通过 | 1.1 |
| 1.4 | 搭主界面骨架：TopBar + HistorySidebar + Terminal 三块占位（先放假数据） | 启动后看到三块布局，与 UI_UX §4.1 线框对得上 | 1.3 |
| 1.5 | 配 ESLint + Prettier + clippy + rustfmt + Husky pre-commit | `git commit` 触发 lint，违规阻止提交 | 1.1 |
| 1.6 | 配 GitHub Actions CI：Win 上跑 `cargo test` + `pnpm test` + `pnpm tauri build`（仅 PR 跑 build） | PR 时绿勾 | 1.5 |

**M1 验收**（2026-04-27）：本地 `pnpm tauri dev` 能启动，看到三栏占位界面；GitHub PR CI 绿勾。

---

## Week 2（4/28–5/04）：M2 PTY 跑通

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 2.1 | Rust 端实现 `pty_manager::pty_spawn` / `pty_write` / `pty_resize` / `pty_kill` + `pty_output` 事件 | 单元测试 spawn `cmd.exe` 能拿到输出 | M1 |
| 2.2 | 暴露为 Tauri commands（`commands::pty`），定义 IPC 类型 | 前端可 invoke | 2.1 |
| 2.3 | 前端 Terminal 组件：xterm.js + addon-fit + 双向桥接到 PTY | 输入 `dir`/`ls` 可见输出 | 2.2 |
| 2.4 | 用 Workbench 选文件夹（dialog.open）后启动 `claude` PTY | 在装了 Claude Code 的机器上能进入对话 | 2.3 |
| 2.5 | 处理 PTY exit 事件：终端区显示「Claude 已退出，点击重启」 | 手动 kill 进程后 UI 提示 | 2.3 |

**M2 验收**（2026-05-04）：选个有 Claude Code 的目录，能在嵌入式终端里和 Claude 对话。

---

## Week 3（5/05–5/11）：M3 检测三件套

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 3.1 | `env_checker::check_claude_installed`（Win 探测路径分支） | 装/未装两种情况返回正确 | M1 |
| 3.2 | `env_checker::check_claude_auth_status`（包装 `claude auth status`） | 已登录/未登录两种返回正确 | 3.1 |
| 3.3 | `net::probe` + `env_checker::check_network`（HEAD api.anthropic.com） | 通/不通两种返回正确 | M1 |
| 3.4 | `env_checker::check_for_updates`（GitHub Releases API，5s 超时静默） | 有新版/无新版/超时三种返回正确 | M1 |
| 3.5 | `commands::env::check_environment` 一次性并行调上面 4 个 | IPC 返回结构体含全部 4 个字段 | 3.1–3.4 |
| 3.6 | TopBar 状态指示灯（network 圆点 + claude 版本 + 齿轮红点） | 可视化 4 项状态 | 3.5 |
| 3.7 | ReadinessWizard 组件：失败项一次展示 + 解决任意一项后局部刷新 | UI_UX §4.2 线框对得上 | 3.5 |
| 3.8 | LoginPromptDialog 一次性弹（与 AppConfig.suppressLoginPrompt 联动） | 勾选"不再提示"持久化 | 3.2 |

**M3 验收**（2026-05-11）：未装 Claude Code 的机器启动 → 弹 ReadinessWizard 显示"未检测到"+网络状态；已装但未登录 → 弹 LoginPromptDialog；全通过 → 进主界面。

---

## Week 4–5（5/12–5/25）：M4 一键安装

> **本里程碑跨 2 周**——Q1.2 决策的代价，重点保证安装成功率 ≥ 90%。

### Week 4：核心流程

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 4.1 | 调研并维护 Anthropic 官方 Win 原生安装包下载 URL（写入常量，加 ADR 注释何时同步） | 文档中明确 URL 来源 | — |
| 4.2 | `env_checker::install_claude_native`：下载 → 流式进度事件 | 单元测试用本地 mock 服务器跑通 | M1 |
| 4.3 | 数字签名校验：拒绝非 "Anthropic, PBC" 主体的安装包 | 单元测试用错误签名样本验证拒绝 | 4.2 |
| 4.4 | spawn 安装包让 Win 弹 UAC，本工具进程不提权 | 集成测试人工验证（Win11 VM） | 4.2 |
| 4.5 | 安装后自动 `claude --version` 验证 | 安装成功后 ReadinessWizard 自动消失 | 4.4 |

### Week 5：UI + 错误回退

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 5.1 | InstallProgressDialog UI：进度条 + 当前步骤文案 + 取消按钮 | 全程进度可见 | 4.2 |
| 5.2 | `cancel_install` 中断下载/安装；恢复回 ReadinessWizard | 中途取消不留垃圾文件 | 5.1 |
| 5.3 | 失败回退：任一步失败 → 显示原因 + "打开官方下载页"按钮 | 网络断开 / 签名错 / UAC 拒绝三种场景都有友好提示 | 5.1 |
| 5.4 | 跨网速回归测试：3G / 慢宽带 / 千兆三档 | 全部能装好或友好失败 | 5.3 |

**M4 验收**（2026-05-25）：在干净 Win10 1809+ VM 上，从启动 ClaudeTool 到 Claude Code 可用 ≤ 5 分钟（含安装），成功率 ≥ 90%。

---

## Week 6（5/26–6/01）：M5 历史会话面板

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 6.1 | `fs::project_session_dir(workdir)`：cwd → URL 编码 → `~/.claude/projects/<encoded>/` | 单测覆盖含中文路径、空格、特殊字符 | M1 |
| 6.2 | `history_parser::list_sessions_quick`：流式读 jsonl 头部，遇关键字段即停 | 100 个会话首屏 ≤ 300ms（CONSTRAINTS §1） | 6.1 |
| 6.3 | `history_parser::refine_session_metadata`：后台异步精确化 turnCount，事件推送 | 列表中 turnCount 由"~"刷新为精确值 | 6.2 |
| 6.4 | HistorySidebar 组件：列表渲染 + 模糊搜索 + 排序（updatedAt DESC） | UI_UX §4.1 对得上 | 6.2 |
| 6.5 | 点击某条 → 写入 `claude --resume <id>` 到当前 PTY 并自动回车 | 续聊能恢复上下文 | M2, 6.4 |
| 6.6 | "继续上次"按钮（`-c`）+ "Fork 一份"按钮（`--fork-session`） | 三按钮分别测通 | 6.5 |
| 6.7 | 损坏 jsonl 容错：解析失败的会话从列表静默跳过 + 日志 warn | 故意造一个损坏 jsonl 不崩溃 | 6.2 |

**M5 验收**（2026-06-01）：选个有 ≥ 5 条历史的项目，列表流畅展示，点任一条能续聊。

---

## Week 7（6/02–6/08）：M6 命令大全 + Skills

### 命令大全 F7

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 7.1 | 整理 `app/src/data/commands.zh-CN.json`：30 条最高频（CLI flags + slash commands） | JSON schema 校验通过；分类齐 | — |
| 7.2 | CommandPanel Drawer 组件：搜索 + Tab 分类 + 列表卡片 | UI_UX §4.3 对得上 | 7.1 |
| 7.3 | 「试一试」：写命令到当前 PTY 但不回车（占位符原样保留） | Q4.4 决策一致 | M2, 7.2 |

### Skills F5

| # | 任务 | 验收 | 依赖 |
|---|---|---|---|
| 7.4 | 同步 `anthropics/skills` 仓库到 `app/src-tauri/src/seed/seed-skills.json`（ADR-015） | 含 ~30 条官方 Skill 元数据 | — |
| 7.5 | `skills_scanner::list_installed_skills`：扫用户级 + 项目级 + 插件级 | 100 个 Skill ≤ 200ms | M1 |
| 7.6 | `skills_scanner::list_recommended_skills`：读种子 + 标"已装/未装" | 与 list_installed 去重 | 7.4, 7.5 |
| 7.7 | SkillsPanel Drawer：[已装] [推荐] 双 Tab；点击展开 SKILL.md | UI_UX §4.4 对得上 | 7.5, 7.6 |
| 7.8 | 「复制安装命令」按钮（不做一键安装） | 点击复制到剪贴板 | 7.7 |

**M6 验收**（2026-06-08）：命令大全和 Skills 抽屉都能打开搜索查看；命令试一试能写入终端。

---

## Week 8（6/09–6/15）：M7 联调 + 体验打磨

| # | 任务 | 验收 |
|---|---|---|
| 8.1 | F9 半自动更新提示完整链路：启动检查 + 红点 + UpdateDialog + 跳转 | 模拟新版 release 后看到红点 |
| 8.2 | 兼容性横幅：Claude Code 版本超出 `SUPPORTED_CCODE_RANGE` 时显示 | 改常量模拟超出 |
| 8.3 | 完整 user journey 走查：未装 Win10 → 装 → 登录 → 选目录 → 续聊 → 试命令 | 5 分钟内完成 |
| 8.4 | 性能优化：冷启动 ≤ 3s、空闲内存 ≤ 150MB | tauri-test 测量 |
| 8.5 | 全 i18n 文案审查：去除组件内硬编码中文 | grep 验证 |
| 8.6 | 错误处理打磨：所有 `AppError` 转中文 toast；无 unhandled rejection | 故意触发各种错验收 |
| 8.7 | 暗色模式 / 100%–200% DPI / 中文输入法 三件套手动测 | TESTING §8 清单过 |
| 8.8 | 写 PRIVACY.md（零数据收集声明） | 根目录新文件 |

**M7 验收**（2026-06-15）：全功能走查通过，性能达标，准备打 Beta 包。

---

## Week 9（6/16–6/22）：M8 Beta 内测

| # | 任务 | 验收 |
|---|---|---|
| 9.1 | 跑发布前 Checklist（ROADMAP "发布前 Checklist"）：种子同步 + 版本回归 + 测试清单 + 包体校验 | 全部 ✅ |
| 9.2 | 录 30 秒「SmartScreen 仍要运行」教学视频 | mp4 + GIF 两版 |
| 9.3 | Build Beta 安装包（`pnpm tauri build`），上传 GitHub Releases pre-release | exe 可下载 |
| 9.4 | 写 Beta 邀请文案 + 反馈表（Google Form / 飞书表单） | 链接可用 |
| 9.5 | 私信 3 名熟人内测 | 至少 2 名当周完成首次启动 |
| 9.6 | 发布招募帖：掘金 + V2EX + 小红书各 1 条 | 收满 5–7 名内测 |
| 9.7 | 收集反馈，优先级排序，修紧急 bug | bug 列表清空 P0/P1 |

**M8 验收**（2026-06-22）：Beta 用户≥ 8 人，无阻塞性 bug，反馈分类完毕。

---

## Week 10（6/23–6/29）：MVP 1.0 发布

| # | 任务 | 验收 |
|---|---|---|
| 10.1 | 修最后一波 P0/P1 bug | bug 列表清零 |
| 10.2 | 写 Release Notes：功能列表 + 已知限制 + 安装步骤 + SmartScreen 说明 + 致谢 | RELEASE_NOTES.md |
| 10.3 | 打 Tag `v1.0.0` + 发布 GitHub Release（正式版，非 pre-release） | Release 页可见 |
| 10.4 | README 完善：动图演示 + 快速开始 + 常见问题 + 贡献指南 | README 完整 |
| 10.5 | 发布社交媒体 + 内测群报喜 + 写复盘文章（可选） | 触达 ≥ 1000 PV |

**MVP 1.0 验收**（2026-06-29）：GitHub Release v1.0.0 公开发布，README 体面，内测用户中至少有 3 人愿意继续用。

---

## 跨周持续任务（每周都要做）

- **每日 standup**（自己跟自己说也行）：今天做什么 / 卡在哪 / 需不需要开 ADR
- **每周五**：更新 ROADMAP §4「当前待办（近 2 周）」+ §5「已完成」
- **每完成一项任务**：commit + push（Day 1 公开仓库决策的代价就是要保持仓库干净）
- **每出现一次架构决策**：先写 ADR 再动代码
- **每周末**：通读 12 份文档有无遗漏（5 分钟扫一遍）

---

## 风险标记（在路线图上特别留意的任务）

- 🔴 **4.4 UAC 行为**：Win 不同版本 UAC 弹法可能不同，VM 测覆盖 Win10 1809 / Win10 22H2 / Win11
- 🔴 **6.2 jsonl 流式解析**：Anthropic 升级 CLI 可能改格式，预留容错
- 🟡 **7.4 anthropics/skills 同步**：仓库结构若变（例：换组织名）需手动更新代码
- 🟡 **8.4 性能**：Bundled WebView2 + xterm.js 可能让冷启动接近 3s 上限
- 🟡 **9.6 公开招募效果**：发了帖没人来怎么办——备用方案：在 Claude Code 中文群发

---

## 与 ROADMAP 的同步要点

- ROADMAP §1 "本周重点" → 每周一改
- ROADMAP §3 里程碑状态 → 完成里程碑当天勾✅
- ROADMAP §4 "近 2 周待办" → 每周五重排
- ROADMAP §5 "已完成" → 大事件当天追加
- ROADMAP §6 "已搁置" → 范围变更时追加（同时开 ADR）
- WORK_PLAN（本文）→ 范围有大变化时（不是任务调整）才回头改
