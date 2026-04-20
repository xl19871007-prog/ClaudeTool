# ClaudeTool

> Claude Code 的小白桌面入口 · Windows 优先 · 中文 UI

**当前状态**：MVP 开发中（2026-04-20 启动，目标 2026-06-29 发布）

---

## 这是什么

ClaudeTool 是一款 **Tauri 2 桌面应用**，把 Anthropic 官方 Claude Code CLI 包装成对中文小白用户友好的工作台：

- 🛠️ **环境助手**：检测 Claude Code 安装、一键安装（Win 原生路径）、检测网络
- 💻 **嵌入式终端**：选文件夹直接在该目录启动 `claude`
- 📜 **历史会话**：扫描 `~/.claude/projects/`，点击一键续聊
- 📚 **命令大全**：内置中文化字典 + 「试一试」自动写入终端
- 🧩 **Skills 管理**：浏览本地已装 + 推荐官方仓库 Skills

## 这不是什么

- ❌ 不是编辑器（请用 Cursor / VSCode）
- ❌ 不接入其他 LLM（仅 Claude Code 周边）
- ❌ 不内置代理 / VPN（网络问题请在 OS 层解决）
- ❌ 不替代 Claude Code 本身的对话能力

---

## 文档

完整开发文档见 [`ClaudeTool开发文档/`](./ClaudeTool开发文档/)：

| 文档 | 内容 |
|---|---|
| [CLAUDE.md](./ClaudeTool开发文档/1.核心文件（必做）/1.CLAUDE.md) | 项目总纲（先读这个） |
| [PRD.md](./ClaudeTool开发文档/1.核心文件（必做）/2.PRD.md) | 产品需求 |
| [TECH_STACK.md](./ClaudeTool开发文档/1.核心文件（必做）/3.TECH_STACK.md) | 技术栈 |
| [ARCHITECTURE.md](./ClaudeTool开发文档/1.核心文件（必做）/4.ARCHITECTURE.md) | 架构设计 |
| [ROADMAP.md](./ClaudeTool开发文档/2.工作规范（强烈建议）/6.（建议材料）ROADMAP.md) | 当前进度 |
| [WORK_PLAN.md](./WORK_PLAN.md) | 10 周执行手册 |
| [MARKET_RESEARCH.md](./MARKET_RESEARCH.md) | 立项调研 |
| [REVIEW_LOG.md](./REVIEW_LOG.md) | 文档评审记录 |

---

## 构建（开发者）

> 完整环境与命令清单见 [WORK_PLAN.md](./WORK_PLAN.md) 的 "Day 1 启动 Checklist"。

```bash
cd app
pnpm install
pnpm tauri dev
```

需要：Node 18+ / Rust stable / pnpm。

---

## License

MIT © ClaudeTool contributors
