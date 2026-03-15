# Requirements: CLIManager

**Defined:** 2026-03-15
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v2.4 Requirements

Requirements for milestone v2.4 Anthropic 模型映射。

### 模型映射

- [ ] **MMAP-01**: Anthropic 协议透传请求在转发前执行模型映射（复用三级优先级：精确匹配 > 默认模型 > 保留原名）
- [ ] **MMAP-02**: Anthropic 透传响应中的 model 字段映射回原始模型名（客户端看到仍是 Claude 模型名）
- [ ] **MMAP-03**: Anthropic 透传流式 SSE 中的 model 字段映射回原始模型名
- [ ] **MMAP-04**: Anthropic 协议 Provider 编辑 UI 显示模型映射配置（默认模型和映射对均为可选，无建议值/placeholder）

## Future Requirements

None — v2.4 scope is focused.

## Out of Scope

| Feature | Reason |
|---------|--------|
| 反向协议转换（OpenAI->Anthropic） | v2.2 只做 Anthropic->OpenAI 方向，反向转换属于 2.x 全功能网关 |
| 流量监控与可视化 | 2.x 全功能网关里程碑 |
| Proxy Failover / Usage 统计 | 2.x 全功能网关里程碑 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| MMAP-01 | Phase 23 | Pending |
| MMAP-02 | Phase 23 | Pending |
| MMAP-03 | Phase 23 | Pending |
| MMAP-04 | Phase 23 | Pending |

**Coverage:**
- v2.4 requirements: 4 total
- Mapped to phases: 4
- Unmapped: 0

---
*Requirements defined: 2026-03-15*
*Last updated: 2026-03-15 — traceability updated after roadmap creation*
