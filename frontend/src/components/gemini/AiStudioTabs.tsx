import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import TabNavigation from "../common/TabNavigation";
import KeySubmitForm from "./KeySubmitForm";
import KeyVisualization from "./KeyVisualization";

const AiStudioTabs: React.FC = () => {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<"submit" | "status">("submit");

  const endpointNative = "http://127.0.0.1:8484/v1/v1beta/generateContent";
  const endpointCli =
    "http://127.0.0.1:8484/gemini-cli/v1/v1beta/generateContent";
  const endpointOai = "http://127.0.0.1:8484/gemini/chat/completions";

  const tabs = [
    { id: "submit", label: t("geminiAiStudio.submit"), color: "purple" },
    { id: "status", label: t("geminiAiStudio.status"), color: "violet" },
  ];

  return (
    <div className="w-full">
      <TabNavigation
        tabs={tabs}
        activeTab={activeTab}
        onTabChange={(tabId) => setActiveTab(tabId as "submit" | "status")}
        className="mb-6"
      />

      {activeTab === "submit" ? <KeySubmitForm /> : <KeyVisualization />}

      <div className="mt-6 space-y-2 text-xs text-gray-400">
        <div className="font-semibold text-gray-300">
          {t("geminiAiStudio.endpoints.title")}
        </div>
        <ul className="space-y-1">
          <li>{t("geminiAiStudio.endpoints.native", { url: endpointNative })}</li>
          <li>{t("geminiAiStudio.endpoints.cli", { url: endpointCli })}</li>
          <li>{t("geminiAiStudio.endpoints.openai", { url: endpointOai })}</li>
        </ul>
      </div>
    </div>
  );
};

export default AiStudioTabs;
