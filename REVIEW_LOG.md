# ClaudeTool 文档评审记录

> **评审日期**：2026-04-20
> **评审范围**：12 份开发文档（位于 `ClaudeTool开发文档/`）
> **评审方法**：Claude 主持，按"战略 → 技术风险 → 数字细节 → 收尾"4 轮提出聚焦问题
> **题数**：22 题（每轮 5–6 题）
> **新增决策**：7 条 ADR（编号 009–015，修订 ADR-004 与 ADR-006）

---

## 评审收益概览

| 维度 | 评审前 | 评审后 |
|---|---|---|
| MVP 工期 | 8 周 | 10 周（一键安装从 V2 提到 MVP 致 +2 周） |
| MVP P0 功能数 | 6 项 | 6 P0 + 1 P1（F9 半自动更新） |
| 安装包大小 | 30MB（理想） | 250MB（含 Bundled WebView2） |
| 启动性能 | ≤ 2s / ≤ 100MB（乐观） | ≤ 3s / ≤ 150MB（务实） |
| 代理处理策略 | 提供配置 UI | 完全不管，用户用 OS VPN |
| Skills 推荐源 | 人工 curate 30 条 | 直接用 anthropics/skills 官方仓库 |
| ADR 总数 | 8 | 15 |
| 开放问题（PRD §7） | 5 个 | 5 个已闭环，2 个推迟 V2 |

---

## 第 1 轮：战略与定位

> **目的**：定大方向，最高杠杆决策先拍板。

### Q1.1 产品命名
- **题面**：当前用 ClaudeTool 偏描述性，是否换个产品名？
- **候选**：A 保持 ClaudeTool / B CCMate / C Claudio / D 小克 / E 自取
- **Claude 倾向**：D（中文向亲和力强）
- **用户决定**：**A 保持 ClaudeTool**
- **影响范围**：无（与现有文档一致，无需 patch）

### Q1.2 一键安装边界
- **题面**：MVP 是引导用户跳官网下载，还是应用内一键安装？
- **候选**：A 跳转引导 / B 应用内一键安装 / C 跳转 + 录屏教程
- **Claude 倾向**：A（守 MVP 范围）
- **用户决定**：**B 应用内一键安装**
- **影响范围**：**最大变化**——PRD F1 重写、ARCHITECTURE env_checker 加 install 接口、ROADMAP 工期 +2 周（→ M4 加里程碑）、新增 ADR-009、CONSTRAINTS 加签名校验与 UAC

### Q1.3 用户画像
- **题面**：是否砍掉次要画像"老王"专聚焦小白？
- **候选**：A 只留小白 / B 保留 + 标主次 / C 换人
- **Claude 倾向**：C
- **用户决定**：**B 保留老王，标为次要画像**
- **影响范围**：PRD §2 加主次标注

### Q1.4 MVP P0 数量
- **题面**：6 项 P0 是否过满，要不要砍 1 项？
- **候选**：A 全保 / B 砍 F5 / C 砍 F7 / D 砍 F5+F7
- **Claude 倾向**：A
- **用户决定**：**A 全保**
- **影响范围**：无

### Q1.5 平台与时间线
- **题面**：MVP 是仅 Win 还是同步出 Mac？
- **候选**：A 严格仅 Win / B Win+Mac 同步 / C 仅 Win 但 V1.1 4 周内
- **Claude 倾向**：A 或 C
- **用户决定**：**A 严格仅 Win**
- **影响范围**：无（与 ADR-003 一致）

### Q1.6 定位语
- **题面**：窄定位 vs 宽定位？
- **候选**：窄"Claude Code 的小白入口" / 宽"小白用的 AI 编程入门工具" / 功能性"Claude Code 中文化客户端"
- **Claude 倾向**：窄
- **用户决定**：**窄定位**
- **影响范围**：CLAUDE.md §3 加原则 #8（不接其他 LLM）；新增 **ADR-010 锁死窄定位**（永远不做清单）

---

## 第 2 轮：技术风险

> **目的**：识别假设可能崩塌的点，提前定 fallback。

### Q2.1 jsonl 格式漂移风险
- **题面**：Claude Code 升级 CLI 改 jsonl 字段时如何应对？
- **候选**：A 内置版本范围 / B 完全宽松 / C A + V2 用户主动上报
- **Claude 倾向**：A
- **用户决定**：**C**
- **影响范围**：CLAUDE.md §3 #3 遥测原则微调；CONSTRAINTS §6 + UI_UX 加兼容性横幅；新增 **ADR-011**

### Q2.2 SmartScreen 警告
- **题面**：MVP 要不要投 EV 证书消除警告？
- **候选**：A 接受现状 + 教学视频 / B EV 证书 / C OV 证书 / D 只发 zip
- **Claude 倾向**：A
- **用户决定**：**A 接受现状**
- **影响范围**：ROADMAP 加发布前 Checklist（含录视频）

### Q2.3 WebView2 分发
- **题面**：嵌入 WebView2 还是 bootstrap 下载？
- **候选**：A Bundled / B Downloader / C System
- **Claude 倾向**：B
- **用户决定**：**A Bundled**
- **影响范围**：包体 30MB → 250MB；PRD §6 / TECH_STACK §2 / CONSTRAINTS §2 同步；新增 **ADR-012**

### Q2.4 代理配置
- **题面**：是否提供代理输入框？
- **候选**：A 输入框无解释 / B 输入框 + tooltip / C 详细教程 / D B + 测试连接
- **Claude 倾向**：D
- **用户决定**：**完全不管，用户走 OS 层 VPN**
- **影响范围**：**第二大变化**——删除 PRD F4 / DATA_MODEL AppConfig.proxy / ARCHITECTURE pty_manager 代理依赖 / UI_UX SettingsDialog 代理项 / TESTING 网络与代理；新增 **ADR-013（局部修订 ADR-004）**

### Q2.5 Skills 种子维护
- **题面**：30 条种子 stars 字段如何同步？
- **候选**：A 人工每版本 / B GitHub Actions 半自动 / C 不维护
- **Claude 倾向**：B
- **用户决定**：**A 人工每版本**
- **影响范围**：ROADMAP 发布前 Checklist 写入"同步种子"任务

---

## 第 3 轮：细节一致性与数字现实性

> **目的**：把"上线发现做不到"的指标提前调到现实。

### Q3.1 性能数字
- **题面**：≤ 2s / ≤ 100MB 是否过激进？
- **候选**：A 维持 / B 放宽 ≤ 3s / ≤ 150MB / C P50/P95 分级
- **Claude 倾向**：B
- **用户决定**：**B 放宽**
- **影响范围**：PRD §6 + CONSTRAINTS §1/§2 + ROADMAP 阶段 1 验收

### Q3.2 启动检测顺序
- **题面**：F1 与 F4 串行还是并行？
- **候选**：A 严格串行 / B 并行 + 统一向导 / C 部分错峰
- **Claude 倾向**：B
- **用户决定**：**B 并行**
- **影响范围**：PRD F1 依赖描述；UI_UX ReadinessWizard 文案

### Q3.3 jsonl 解析策略
- **题面**：100 个会话 ≤ 300ms 怎么实现？
- **候选**：A 只读头 / B 读全文件 / C 流式即停 / D C + 后台异步精确化
- **Claude 倾向**：D
- **用户决定**：**D**
- **影响范围**：ARCHITECTURE history_parser 拆双接口；DATA_MODEL turnCount 标异步；CONSTRAINTS §1 加流式说明；新增 **ADR-014**

### Q3.4 命令大全数量
- **题面**：MVP 字典覆盖多少条？
- **候选**：A 现整理 100 条 / B 实现时再整 / C MVP 30 条 + V1.1 补
- **Claude 倾向**：C
- **用户决定**：**C MVP 30 条**
- **影响范围**：PRD F7 + ROADMAP MVP 验收加"≥ 30 条"

### Q3.5 Beta 内测渠道
- **题面**：5–10 名小白用户从哪招？
- **候选**：A 熟人 / B 公开 / C 极客群 / D 组合
- **Claude 倾向**：D
- **用户决定**：**D 组合**
- **影响范围**：ROADMAP M8 写明"3 熟人 + 5–7 公开"

---

## 第 4 轮：开放问题清扫

> **目的**：清空 PRD §7 的开放问题；定 Day-1 启动配置。

### Q4.1 Claude.ai 登录入口
- **题面**：未登录怎么处理？
- **候选**：A 不管 / B 常驻按钮 / C 检测到弹一次提示
- **Claude 倾向**：C
- **用户决定**：**C 一次性提示**
- **影响范围**：PRD F1 加 auth status 检测 + LoginPromptDialog；DATA_MODEL AppConfig 加 `suppressLoginPrompt`；ARCHITECTURE env_checker 加 `check_claude_auth_status`

### Q4.2 自动更新
- **题面**：MVP 自动更新机制？
- **候选**：A 完全手动 / B 内置自动 / C 半自动（启动检查 + 红点 + 跳 Releases）
- **Claude 倾向**：C
- **用户决定**：**C 半自动**
- **影响范围**：PRD 新增 F9 [P1]；ARCHITECTURE 加 `check_for_updates`；CONSTRAINTS §4 出站请求白名单加 GitHub API；UI_UX 加 UpdateDialog + 顶栏红点

### Q4.3 Skills 种子源
- **题面**：30 条种子谁选？
- **候选**：A 人工 curate / B Claude 起稿 + 人工审 / C 直接用 anthropics/skills
- **Claude 倾向**：B
- **用户决定**：**C 直接用官方仓库**
- **影响范围**：新增 **ADR-015（修订 ADR-006）**；ROADMAP 发布前 Checklist 改文案

### Q4.4 试一试参数填空
- **题面**：命令参数怎么处理？
- **候选**：A 占位符原样 / B 光标定位 / C 智能填空
- **Claude 倾向**：A
- **用户决定**：**A 不做填空**
- **影响范围**：PRD §7 闭环此问题（无新增需求）

### Q4.5 开源策略
- **题面**：Day 1 公开还是 MVP 后开源？
- **候选**：A Day 1 公开 / B MVP 后 / C 永久私有
- **Claude 倾向**：A
- **用户决定**：**A Day 1 公开**
- **影响范围**：ROADMAP §4 当前待办加"创建公开 GitHub 仓库 + LICENSE(MIT)"；CLAUDE.md §5 加仓库布局说明

### Q4.6 Git 仓库布局
- **题面**：仓库根在哪里？
- **候选**：A `D:\ClaudeTool\` 整体 / B `app/` 子目录 / C Monorepo 重组
- **Claude 倾向**：A + .gitignore 排模板
- **用户决定**：**A**
- **影响范围**：ROADMAP §4 加 ".gitignore 排 AI开发文档模板/"；CLAUDE.md §5 加布局说明

---

## 累计文档变更总表

| 文档 | 评审总变更次数 | 主要内容 |
|---|---|---|
| 1.CLAUDE.md | 4 处 | §3 加原则 #8（不接其他 LLM）+ #3 遥测微调；§5 加仓库布局；§6 提一键安装 |
| 2.PRD.md | 9 处 | F1 重写（一键安装 + auth）；新增 F9（半自动更新）；F4 删代理；F7 加数量；§2 画像主次；§5 Out of Scope；§6 性能/包体；§7 开放问题闭环 |
| 3.TECH_STACK.md | 1 处 | WebView2 明确 Bundled |
| 4.ARCHITECTURE.md | 5 处 | env_checker 加 install/auth/update 接口；pty_manager 删代理依赖；history_parser 拆双接口；数据流 A 改并行；§7 加兼容性横幅 |
| 5.CODE_STYLE.md | 0 | — |
| 6.ROADMAP.md | 6 处 | 工期 8 → 10 周；M4 改一键安装；M8 内测渠道；阶段 1 验收数字；§4 当前待办（仓库 + 模板排除）；发布前 Checklist |
| 7.DOMAIN.md | 0 | — |
| 8.DATA_MODEL.md | 3 处 | AppConfig 删 proxy.* + 加 suppressLoginPrompt + lastSeenVersion；turnCount 标异步精确化 |
| 9.UI_UX.md | 4 处 | SettingsDialog 改字段；ReadinessWizard 文案；加兼容性横幅 / LoginPromptDialog / UpdateDialog；TopBar 红点说明 |
| 10.TESTING.md | 1 处 | 网络与代理 → 网络（OS VPN 测试） |
| 11.CONSTRAINTS.md | 5 处 | 性能/内存/包体放宽；§3 不管代理 + 安装包签名 + UAC；§4 出站请求白名单加 2 项；§6 兼容版本白名单 |
| 12.DECISIONS.md | 7 新 + 2 修订 | 新增 ADR-009 至 ADR-015；ADR-004 与 ADR-006 标局部修订 |

---

## ADR 全表（评审后状态）

| 编号 | 决策 | 状态 |
|---|---|---|
| ADR-001 | 选用 Tauri 2 而非 Electron | 已接受 |
| ADR-002 | 暂缓 Token 配额监控（PRD F3） | 已接受 |
| ADR-003 | MVP 仅 Win + 仅中文 | 已接受 |
| ADR-004 | 不主动推广区域限制规避 | 已接受（局部由 ADR-013 修订） |
| ADR-005 | 嵌入式终端走 tauri-plugin-pty | 已接受 |
| ADR-006 | Skills 用静态种子 | 已接受（局部由 ADR-015 修订） |
| ADR-007 | 对话引导走规则引擎，推 V2 | 已接受 |
| ADR-008 | 文档体系沿用 4 层 12 模板 | 已接受 |
| **ADR-009** | **MVP 包含一键安装 Win 原生路径** | 已接受（评审 Q1.2） |
| **ADR-010** | **锁定窄定位「Claude Code 的小白入口」** | 已接受（评审 Q1.6） |
| **ADR-011** | **jsonl 兼容版本范围 + V2 主动上报** | 已接受（评审 Q2.1） |
| **ADR-012** | **WebView2 用 Bundled 模式** | 已接受（评审 Q2.3） |
| **ADR-013** | **MVP 不内置代理配置** | 已接受（评审 Q2.4，局部修订 ADR-004） |
| **ADR-014** | **历史会话流式 + 异步精确化** | 已接受（评审 Q3.3） |
| **ADR-015** | **Skills 种子用 anthropics/skills 官方仓库** | 已接受（评审 Q4.3，局部修订 ADR-006） |

---

## 评审方法论备忘（供后续项目复用）

1. **谁主持**：Claude 主持，用户决策。用户自审会"一眼带过"漏关键点。
2. **轮次设计**：4 轮，每轮 5–6 题。
   - 第 1 轮战略（命名、定位、范围、平台）
   - 第 2 轮技术风险（API 变化、签名、依赖、合规）
   - 第 3 轮数字（性能、容量、时间）
   - 第 4 轮收尾（开放问题、仓库布局、发布策略）
3. **每题格式**：题面 → 候选 → Claude 倾向 → 影响范围 → 用户拍板
4. **答完即 patch**：每轮答完批量修改文档，再开下一轮，避免堆积
5. **保留记录**：本 REVIEW_LOG.md 即为产出，未来回顾"为什么这么定"用得着
