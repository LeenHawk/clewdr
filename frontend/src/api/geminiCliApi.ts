import {
  GeminiCliCredentialInfo,
  GeminiCliCredentialPayload,
} from "../types/geminiCli.types";

function getToken() {
  return localStorage.getItem("authToken") || "";
}

export async function getGeminiCliCredentials(): Promise<
  GeminiCliCredentialInfo[]
> {
  const response = await fetch("/api/gemini/cli/credentials", {
    headers: {
      Authorization: `Bearer ${getToken()}`,
      "Content-Type": "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(`Failed to load Gemini CLI credentials: ${response.status}`);
  }

  return (await response.json()) as GeminiCliCredentialInfo[];
}

export async function addGeminiCliCredential(
  credential: GeminiCliCredentialPayload,
) {
  const response = await fetch("/api/gemini/cli/credential", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${getToken()}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ credential }),
  });

  if (!response.ok) {
    throw new Error(`Failed to add Gemini CLI credential: ${response.status}`);
  }

  return response;
}

export async function deleteGeminiCliCredential(
  clientId: string,
  projectId: string,
) {
  const response = await fetch("/api/gemini/cli/credential", {
    method: "DELETE",
    headers: {
      Authorization: `Bearer ${getToken()}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ client_id: clientId, project_id: projectId }),
  });

  if (!response.ok) {
    throw new Error(
      `Failed to delete Gemini CLI credential: ${response.status}`,
    );
  }

  return response;
}

