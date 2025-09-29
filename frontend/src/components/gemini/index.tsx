import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import TabNavigation from "../common/TabNavigation";
import AiStudioTabs from "./AiStudioTabs";
import VertexTabs from "./vertex/VertexTabs";
import CliTabs from "./cli/CliTabs";

const GeminiTabs: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"aistudio" | "vertex" | "cli">(
    "aistudio"
  );

  const tabs = [
    { id: "aistudio", label: t("geminiTabs.aistudio"), color: "purple" },
    { id: "cli", label: t("geminiTabs.cli"), color: "indigo" },
    { id: "vertex", label: t("geminiTabs.vertex"), color: "cyan" },
  ];

  return (
    <div className="w-full">
      <TabNavigation
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(tabId) =>
          setActiveTab(tabId as "aistudio" | "vertex" | "cli")
        }
        className="mb-6"
      />

      {activeTab === "aistudio" ? (
        <AiStudioTabs />
      ) : activeTab === "cli" ? (
        <CliTabs />
      ) : (
        <VertexTabs />
      )}
    </div>
  );
};

export default GeminiTabs;
