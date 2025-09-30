// frontend/src/types/cookie.types.ts
export interface CookieStatus {
  cookie: string;
  reset_time: number | null;
  supports_claude_1m?: boolean | null;
  count_tokens_allowed?: boolean | null;
  total_input_tokens?: number;
  total_output_tokens?: number;
  window_input_tokens?: number;
  window_output_tokens?: number;
  // Ephemeral quota utilizations (percent), attached by /api/cookies only
  session_utilization?: number;
  seven_day_utilization?: number;
  seven_day_opus_utilization?: number;
  // Resets at timestamps (ISO8601), attached by /api/cookies only
  session_resets_at?: string | null;
  seven_day_resets_at?: string | null;
  seven_day_opus_resets_at?: string | null;
}

export interface UselessCookie {
  cookie: string;
  reason: string | any;
}

export interface CookieStatusInfo {
  valid: CookieStatus[];
  exhausted: CookieStatus[];
  invalid: UselessCookie[];
}

export interface CookieFormState {
  cookie: string;
  isSubmitting: boolean;
  status: {
    type: "idle" | "success" | "error";
    message: string;
  };
}
