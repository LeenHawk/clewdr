import React from "react";
import { useTranslation } from "react-i18next";
import { GeminiCliCredentialInfo } from "../../../types/geminiCli.types";
import DeleteButton from "../DeleteButton";

interface CliCredentialListProps {
  loading: boolean;
  credentials: GeminiCliCredentialInfo[];
  deletingKey: string | null;
  onDelete: (key: string) => Promise<void>;
}

const CliCredentialList: React.FC<CliCredentialListProps> = ({
  loading,
  credentials,
  deletingKey,
  onDelete,
}) => {
  const { t } = useTranslation();
  if (loading) {
    return <p className="text-sm text-gray-400">{t("common.loading")}</p>;
  }

  if (!credentials.length) {
    return <p className="text-sm text-gray-400">{t("geminiCli.empty")}</p>;
  }

  return (
    <div className="space-y-3">
      {credentials.map((cred) => {
        const key = `${cred.client_id}::${cred.project_id}`;
        return (
          <div
            key={key}
            className="flex items-center justify-between rounded-md border border-gray-700/70 bg-gray-900/40 px-3 py-2"
          >
            <div>
              <p className="text-sm font-medium text-gray-200">
                {cred.project_id}
              </p>
              <p className="text-xs text-gray-400">
                {t("geminiCli.table.clientId")}: {cred.client_id}
              </p>
              <p className="text-xs text-gray-500">
                {t("geminiCli.table.expiry")}: {cred.expiry
                  ? new Date(cred.expiry).toLocaleString()
                  : t("common.unknown")}
              </p>
            </div>
            <DeleteButton
              keyString={key}
              onDelete={onDelete}
              isDeleting={deletingKey === key}
            />
          </div>
        );
      })}
    </div>
  );
};

export default CliCredentialList;
