import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import Button from "../common/Button";
import StatusMessage from "../common/StatusMessage";
import { codexStartAuth, codexTokens, codexLogout } from "../../api";

const CodexPanel: React.FC = () => {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<any>(null);

  const refresh = async () => {
    setError(null);
    setLoading(true);
    try {
      const s = await codexTokens();
      setStatus(s);
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const onStart = async () => {
    setError(null);
    setLoading(true);
    try {
      const { auth_url } = await codexStartAuth();
      window.open(auth_url, "_blank");
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setLoading(false);
    }
  };

  const onLogout = async () => {
    setError(null);
    setLoading(true);
    try {
      await codexLogout();
      await refresh();
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-4">
      {error && <StatusMessage type="error" message={error} />}

      <div className="bg-gray-700 p-4 rounded-lg">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-white font-medium">{t("codex.title")}</h3>
          <div className="space-x-2">
            <Button onClick={refresh} disabled={loading} variant="secondary">
              {t("common.refresh")}
            </Button>
            <Button onClick={onStart} disabled={loading}>
              {t("codex.startLogin")}
            </Button>
            <Button onClick={onLogout} disabled={loading} variant="danger">
              {t("codex.logout")}
            </Button>
          </div>
        </div>
        <p className="text-gray-300 text-sm">{t("codex.desc")}</p>
      </div>

      <div className="bg-gray-700 p-4 rounded-lg">
        <h4 className="text-white font-medium mb-2">{t("codex.status")}</h4>
        {status ? (
          <div className="text-gray-300 text-sm space-y-1">
            <div>
              {t("codex.authenticated")}:
              <span className="ml-1 font-mono">{String(status.authenticated)}</span>
            </div>
            <div>
              {t("codex.accountId")}:
              <span className="ml-1 font-mono">{status.account_id || "-"}</span>
            </div>
            <div>
              {t("codex.lastRefresh")}:
              <span className="ml-1 font-mono">{status.last_refresh || "-"}</span>
            </div>
          </div>
        ) : (
          <div className="text-gray-400 text-sm">{t("codex.noStatus")}</div>
        )}
      </div>
    </div>
  );
};

export default CodexPanel;

