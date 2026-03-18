# Requirements: CLIManager

**Defined:** 2026-03-17
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v2.6 Requirements

Requirements for v2.6 流量监控。Each maps to roadmap phases.

### 存储基础设施

- [x] **STORE-01**: 应用启动时自动初始化 SQLite 数据库（WAL 模式，路径在 app_data_dir，非 iCloud）
- [x] **STORE-02**: schema 迁移机制确保未来加字段安全（rusqlite_migration）
- [ ] **STORE-03**: 代理请求完成后通过 mpsc channel 非阻塞发送日志，后台 task 写入 SQLite 并 emit 到前端
- [ ] **STORE-04**: 定时清理任务聚合超过 24h 的明细为每日统计，删除超过 7d 的统计数据

### 数据采集

- [ ] **COLLECT-01**: 记录每个代理请求的基础元数据（时间戳、CLI、Provider、方法、路径、状态码、总耗时、TTFB、是否流式、请求模型名）
- [ ] **COLLECT-02**: 非流式响应直接从 body 提取 input/output token 用量
- [ ] **COLLECT-03**: 流式 SSE 响应在 stream 结束后提取 token 用量（支持 Anthropic、OpenAI Chat Completions、OpenAI Responses 三种格式）
- [ ] **COLLECT-04**: 请求失败时记录错误信息，成功时记录 stop_reason

### 实时日志

- [ ] **LOG-01**: 后台写入 SQLite 后通过 Tauri emit 实时推送日志条目到前端
- [ ] **LOG-02**: 独立流量监控页面展示实时日志表格（时间、Provider、模型、状态码、token、耗时等列）
- [ ] **LOG-03**: 日志表格支持按 Provider 筛选，缺省显示全部

### 统计分析

- [ ] **STAT-01**: 统计摘要卡片展示总请求数、总 input/output token、成功率
- [ ] **STAT-02**: 按 Provider 聚合表格展示各 Provider 的请求数、token 用量、平均耗时
- [ ] **STAT-03**: 按时间聚合表格展示每小时/每天的请求数、token 量等
- [ ] **STAT-04**: 趋势图表（recharts）可视化时间维度的流量变化

## Future Requirements

Deferred to v2.7+. Tracked but not in current roadmap.

### 高级监控

- **ADV-01**: 费用估算（cost_usd）— 需维护价格表
- **ADV-02**: 实时告警与阈值配置
- **ADV-03**: 导出报表（JSON/CSV）
- **ADV-04**: first_token_ms（流式请求到第一个内容 token 的时间，区别于 TTFB）

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| 请求/响应 body 完整记录 | 隐私风险 + 存储量巨大 |
| SQLite 放 iCloud Drive | WAL + iCloud 最终一致性 = 数据损坏 |
| 无清理策略的无限日志保留 | 磁盘空间不可控 |
| 费用计算 (cost_usd) | 需维护价格表，复杂度高，defer v2.7+ |
| 实时告警 | 需阈值配置 UI + macOS 通知权限，defer v2.7+ |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| STORE-01 | Phase 26 | Complete |
| STORE-02 | Phase 26 | Complete |
| STORE-03 | Phase 27 | Pending |
| STORE-04 | Phase 30 | Pending |
| COLLECT-01 | Phase 27 | Pending |
| COLLECT-02 | Phase 27 | Pending |
| COLLECT-03 | Phase 28 | Pending |
| COLLECT-04 | Phase 27 | Pending |
| LOG-01 | Phase 27 | Pending |
| LOG-02 | Phase 29 | Pending |
| LOG-03 | Phase 29 | Pending |
| STAT-01 | Phase 29 | Pending |
| STAT-02 | Phase 30 | Pending |
| STAT-03 | Phase 30 | Pending |
| STAT-04 | Phase 30 | Pending |

**Coverage:**
- v2.6 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0

---
*Requirements defined: 2026-03-17*
*Last updated: 2026-03-17 — traceability filled after roadmap creation*
