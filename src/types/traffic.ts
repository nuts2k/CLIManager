// TrafficLog 接口 — 与后端 TrafficLogPayload 字段一一对应（含 type 字段）
// 后端来源：src-tauri/src/traffic/log.rs:38-60

/** 按 Provider 聚合统计（对应后端 rollup.rs 的 ProviderStat） */
export interface ProviderStat {
  provider_name: string;
  request_count: number;
  success_count: number;
  total_input_tokens: number;
  total_output_tokens: number;
  cache_triggered_count: number;
  cache_hit_count: number;
  total_cache_read_tokens: number;
  sum_ttfb_ms: number;
  sum_duration_ms: number;
}

/** 按时间聚合趋势点（对应后端 rollup.rs 的 TimeStat） */
export interface TimeStat {
  /** "HH:00" 或 "YYYY-MM-DD" */
  label: string;
  request_count: number;
  /** input + output 合计 */
  total_tokens: number;
}

/** 统计时间范围类型 */
export type TimeRange = "24h" | "7d";

export interface TrafficLog {
  /** 事件类型：新增 / 更新 / 历史 */
  type: "new" | "update" | "history";
  /** 日志行 ID（SQLite rowid） */
  id: number;
  /** 请求开始时间（Unix ms） */
  created_at: number;
  /** Provider 名称 */
  provider_name: string;
  /** CLI 标识（claude / codex） */
  cli_id: string;
  /** HTTP 方法 */
  method: string;
  /** 请求路径 */
  path: string;
  /** HTTP 状态码 */
  status_code: number | null;
  /** 是否为流式请求（0 = 否，1 = 是） */
  is_streaming: number;
  /** 客户端请求的模型名 */
  request_model: string | null;
  /** 上游实际使用的模型名 */
  upstream_model: string | null;
  /** 协议类型（anthropic / openai_chat / openai_responses） */
  protocol_type: string;
  /** 输入 token 数 */
  input_tokens: number | null;
  /** 输出 token 数 */
  output_tokens: number | null;
  /** 缓存创建 token 数 */
  cache_creation_tokens: number | null;
  /** 缓存读取 token 数 */
  cache_read_tokens: number | null;
  /** 首字节时延（ms） */
  ttfb_ms: number | null;
  /** 总耗时（ms） */
  duration_ms: number | null;
  /** 停止原因 */
  stop_reason: string | null;
  /** 错误信息 */
  error_message: string | null;
}
