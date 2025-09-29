export interface GeminiCliCredentialInfo {
  client_id: string;
  project_id: string;
  scopes: string[];
  expiry?: string;
  has_refresh_token: boolean;
}

export interface GeminiCliCredentialPayload {
  client_id: string;
  client_secret: string;
  token: string;
  refresh_token: string;
  scopes?: string[];
  token_uri: string;
  project_id: string;
  expiry?: string;
}

