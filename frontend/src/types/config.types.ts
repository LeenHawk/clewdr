export interface ConfigData {
  // Server settings
  ip: string;
  port: number;

  // Vertex settings
  vertex: VertexConfig;

  // App settings
  check_update: boolean;
  auto_update: boolean;

  // Network settings
  password: string;
  admin_password: string;
  proxy: string | null;
  rproxy: string | null;

  // API settings
  max_retries: number;
  preserve_chats: boolean;
  web_search: boolean;
  enable_web_count_tokens: boolean;

  // Cookie settings
  skip_first_warning: boolean;
  skip_second_warning: boolean;
  skip_restricted: boolean;
  skip_non_pro: boolean;
  skip_rate_limit: boolean;
  skip_normal_pro: boolean;

  // Prompt configurations
  use_real_roles: boolean;
  custom_h: string | null;
  custom_a: string | null;
  custom_prompt: string;

  // Persistence
  persistence?: PersistenceConfig;
}

export interface StorageStatusDetails {
  driver?: string;
  sqlite_path?: string;
  database_url?: string;
}

export interface StorageStatus {
  enabled?: boolean;
  mode?: PersistenceMode;
  healthy?: boolean;
  details?: StorageStatusDetails;
  // top-level latency in DB mode
  latency_ms?: number;
  last_write_ts?: number;
  total_writes?: number;
  avg_write_ms?: number;
  failure_ratio?: number;
  retry_count?: number;
  write_error_count?: number;
  error?: string;
  last_error?: string;
  tx?: {
    begin?: number;
    commit?: number;
    rollback?: number;
    open?: number;
    last_tx_ts?: number;
  };
  conn?: {
    last_conn_ts?: number;
    success?: number;
    error?: number;
    pool_config?: {
      max_connections?: number;
      min_connections?: number;
      connect_timeout_ms?: number;
      acquire_timeout_ms?: number;
      idle_timeout_ms?: number;
    } | null;
  };
}

interface VertexConfig {
  credential?: string | null;
  credentials?: { client_email: string }[];
  [key: string]: unknown;
}

export type PersistenceMode = "file" | "sqlite" | "postgres" | "mysql";

export interface PersistenceConfig {
  mode: PersistenceMode;
  database_url?: string | null;
  sqlite_path?: string | null;
}

export interface ConfigState {
  config: ConfigData | null;
  originalPassword: string;
  originalAdminPassword: string;
  loading: boolean;
  saving: boolean;
  error: string;
}
