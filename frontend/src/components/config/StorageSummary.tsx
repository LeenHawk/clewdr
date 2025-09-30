import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { StorageStatus, PersistenceMode } from "../../types/config.types";

interface StorageSummaryProps {
  status: StorageStatus | null;
  mode?: PersistenceMode;
}

export function StorageSummary({ status, mode }: StorageSummaryProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  const resolvedMode = mode ?? "file";
  const modeLabel = t(`config.storage.modes.${resolvedMode}`, {
    defaultValue: resolvedMode,
  });

  if (!status) {
    return (
      <div className="space-y-2">
        <div className="text-md font-medium text-cyan-300">
          {t("config.storage.modeLabel")}
        </div>
        <div className="text-white text-sm font-medium">{modeLabel}</div>
      </div>
    );
  }

  const infoLines: string[] = useMemo(() => {
    const lines: string[] = [];
    if (status.details?.driver) {
      lines.push(t("config.storage.driver", { driver: status.details.driver }));
    }
    // latency may appear at top-level (DB) or details (legacy)
    const latency = typeof status.latency_ms === "number" ? status.latency_ms : status.details?.latency_ms;
    if (typeof latency === "number") {
      lines.push(t("config.storage.latency", { latency }));
    }
    if (status.details?.sqlite_path) {
      lines.push(t("config.storage.sqlitePath", { path: status.details.sqlite_path }));
    }
    if (status.details?.database_url) {
      lines.push(t("config.storage.databaseUrl", { url: status.details.database_url }));
    }
    if (typeof status.last_write_ts === "number" && status.last_write_ts > 0) {
      lines.push(
        t("config.storage.lastWrite", { time: new Date(status.last_write_ts * 1000).toLocaleString() })
      );
    }
    if (typeof status.total_writes === "number") {
      lines.push(t("config.storage.totalWrites", { count: status.total_writes }));
    }
    if (typeof status.avg_write_ms === "number") {
      lines.push(t("config.storage.avgWriteMs", { ms: status.avg_write_ms.toFixed(2) }));
    }
    if (typeof status.failure_ratio === "number") {
      lines.push(
        t("config.storage.failureRatio", {
          ratio: (status.failure_ratio * 100).toFixed(2),
        })
      );
    }
    if (typeof status.retry_count === "number") {
      lines.push(t("config.storage.retryCount", { count: status.retry_count }));
    }
    if (typeof status.write_error_count === "number") {
      lines.push(t("config.storage.writeErrors", { count: status.write_error_count }));
    }
    if (status.error) {
      lines.push(t("config.storage.error", { error: status.error }));
    }
    if (status.last_error) {
      lines.push(t("config.storage.lastError", { error: status.last_error }));
    }

    // DB-only extended metrics (tx/conn)
    const isDbMode = (status.mode && status.mode !== "file") || status.enabled;
    if (isDbMode) {
      if (status.tx) {
        const { begin, commit, rollback, open, last_tx_ts } = status.tx;
        if (typeof begin === "number" || typeof commit === "number" || typeof rollback === "number") {
          lines.push(
            t("config.storage.tx.summary", {
              defaultValue: `tx: begin=${begin ?? 0}, commit=${commit ?? 0}, rollback=${rollback ?? 0}, open=${open ?? 0}`,
              begin: begin ?? 0,
              commit: commit ?? 0,
              rollback: rollback ?? 0,
              open: open ?? 0,
            })
          );
        }
        if (typeof last_tx_ts === "number" && last_tx_ts > 0) {
          lines.push(
            t("config.storage.tx.last", {
              defaultValue: `last tx: ${new Date(last_tx_ts * 1000).toLocaleString()}`,
              time: new Date(last_tx_ts * 1000).toLocaleString(),
            })
          );
        }
      }
      if (status.conn) {
        const { last_conn_ts, success, error, pool_config } = status.conn;
        if (typeof success === "number" || typeof error === "number") {
          lines.push(
            t("config.storage.conn.summary", {
              defaultValue: `conn: ok=${success ?? 0}, err=${error ?? 0}`,
              ok: success ?? 0,
              err: error ?? 0,
            })
          );
        }
        if (typeof last_conn_ts === "number" && last_conn_ts > 0) {
          lines.push(
            t("config.storage.conn.last", {
              defaultValue: `last conn: ${new Date(last_conn_ts * 1000).toLocaleString()}`,
              time: new Date(last_conn_ts * 1000).toLocaleString(),
            })
          );
        }
        if (pool_config) {
          lines.push(
            t("config.storage.conn.pool", {
              defaultValue:
                `pool: max=${pool_config.max_connections ?? "-"}, min=${pool_config.min_connections ?? "-"}, ` +
                `connect=${pool_config.connect_timeout_ms ?? "-"}ms, acquire=${pool_config.acquire_timeout_ms ?? "-"}ms, idle=${pool_config.idle_timeout_ms ?? "-"}ms`,
            })
          );
        }
      }
    }

    return lines;
  }, [status, t]);

  const hasDetails = infoLines.length > 0;

  return (
    <div className="space-y-2">
      <div className="text-md font-medium text-cyan-300">
        {t("config.storage.modeLabel")}
      </div>
      <div className="text-white text-sm font-medium">{modeLabel}</div>
      <div className="text-xs text-gray-400">
        {t("config.storage.healthLabel")}: {" "}
        {status.healthy
          ? t("config.storage.health.ok")
          : t("config.storage.health.down")}
      </div>
      {hasDetails && (
        <button
          type="button"
          onClick={() => setExpanded((prev) => !prev)}
          className="mt-1 text-xs text-gray-400 underline hover:text-gray-200 transition-colors block w-fit"
        >
          {expanded
            ? t("config.storage.details.hide")
            : t("config.storage.details.show")}
        </button>
      )}
      {expanded && hasDetails && (
        <div className="text-xs text-gray-400 space-y-1">
          {infoLines.map((line, index) => (
            <div key={index} className="break-words">
              {line}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
