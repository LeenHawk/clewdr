import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import TabNavigation from "../../common/TabNavigation";
import CliCredentialUpload from "./CliCredentialUpload";
import CliCredentialList from "./CliCredentialList";
import {
  addGeminiCliCredential,
  getGeminiCliCredentials,
  deleteGeminiCliCredential,
} from "../../../api/geminiCliApi";
import {
  GeminiCliCredentialInfo,
  GeminiCliCredentialPayload,
} from "../../../types/geminiCli.types";
import { toast } from "react-hot-toast";

const CliTabs: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"upload" | "status">("upload");
  const [credentials, setCredentials] = useState<
    GeminiCliCredentialInfo[]
  >([]);
  const [loading, setLoading] = useState(false);
  const [deletingKey, setDeletingKey] = useState<string | null>(null);

  const tabs = useMemo(
    () => [
      { id: "upload", label: t("geminiCli.submit"), color: "indigo" },
      { id: "status", label: t("geminiCli.status"), color: "violet" },
    ],
    [t],
  );

  const loadCredentials = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getGeminiCliCredentials();
      setCredentials(data);
    } catch (error) {
      console.error("Failed to load Gemini CLI credentials", error);
      toast.error(t("geminiCli.errors.load"));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    loadCredentials();
  }, [loadCredentials]);

  const handleUpload = useCallback(
    async (credential: GeminiCliCredentialPayload) => {
      try {
        await addGeminiCliCredential(credential);
        toast.success(t("geminiCli.notifications.uploaded"));
        await loadCredentials();
      } catch (error) {
        console.error("Failed to upload CLI credential", error);
        toast.error(t("geminiCli.errors.upload"));
      }
    },
    [loadCredentials, t],
  );

  const handleDelete = useCallback(
    async (key: string) => {
      const [clientId, projectId] = key.split("::");
      const deleting = `${clientId}::${projectId}`;
      setDeletingKey(deleting);
      try {
        await deleteGeminiCliCredential(clientId, projectId);
        toast.success(t("geminiCli.notifications.deleted"));
        await loadCredentials();
      } catch (error) {
        console.error("Failed to delete CLI credential", error);
        toast.error(t("geminiCli.errors.delete"));
      } finally {
        setDeletingKey(null);
      }
    },
    [loadCredentials, t],
  );

  return (
    <div className="w-full">
      <TabNavigation
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(tabId) => setActiveTab(tabId as "upload" | "status")}
        className="mb-6"
      />

      {activeTab === "upload" ? (
        <CliCredentialUpload onSubmit={handleUpload} />
      ) : (
        <CliCredentialList
          loading={loading}
          credentials={credentials}
          onDelete={handleDelete}
          deletingKey={deletingKey}
        />
      )}
    </div>
  );
};

export default CliTabs;
