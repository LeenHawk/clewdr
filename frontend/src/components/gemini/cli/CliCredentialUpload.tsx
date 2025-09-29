import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "react-hot-toast";
import FormInput from "../../common/FormInput";
import { GeminiCliCredentialPayload } from "../../../types/geminiCli.types";

interface CliCredentialUploadProps {
  onSubmit: (credential: GeminiCliCredentialPayload) => Promise<void>;
}

const CliCredentialUpload: React.FC<CliCredentialUploadProps> = ({ onSubmit }) => {
  const { t } = useTranslation();
  const [rawInput, setRawInput] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [selectedFile, setSelectedFile] = useState<string>("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const safeParse = (content: string): GeminiCliCredentialPayload | null => {
    try {
      const parsed = JSON.parse(content);
      return parsed as GeminiCliCredentialPayload;
    } catch (error) {
      console.error("Failed to parse CLI credential", error);
      return null;
    }
  };

  const handleFile = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) {
      setSelectedFile("");
      return;
    }

    try {
      const text = await file.text();
      setRawInput(text);
      setSelectedFile(file.name);
      toast.success(t("geminiCli.notifications.fileLoaded"));
    } catch (error) {
      console.error("Failed to read CLI credential file", error);
      toast.error(t("geminiCli.errors.readFile"));
    }
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!rawInput.trim()) {
      toast.error(t("geminiCli.errors.empty"));
      return;
    }

    const parsed = safeParse(rawInput);
    if (!parsed) {
      toast.error(t("geminiCli.errors.parse"));
      return;
    }

    if (!parsed.project_id || !parsed.client_id) {
      toast.error(t("geminiCli.errors.missingFields"));
      return;
    }

    setIsSubmitting(true);
    try {
      await onSubmit(parsed);
      setRawInput("");
      setSelectedFile("");
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    } catch (error) {
      console.error("Failed to upload CLI credential", error);
      toast.error(t("geminiCli.errors.upload"));
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <p className="text-sm text-gray-400">{t("geminiCli.form.instructions")}</p>

      <FormInput
        id="gemini-cli-credential"
        name="gemini-cli-credential"
        value={rawInput}
        onChange={(event) => setRawInput(event.target.value)}
        placeholder={t("geminiCli.form.placeholder") || ""}
        label={t("geminiCli.form.jsonLabel")}
        isTextarea
        rows={10}
        disabled={isSubmitting}
      />

      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div className="flex items-center gap-3">
          <input
            ref={fileInputRef}
            type="file"
            accept="application/json"
            onChange={handleFile}
            className="hidden"
          />
          <button
            type="button"
            className="px-4 py-2 rounded-md text-sm font-medium bg-gray-700 hover:bg-gray-600 text-gray-100 transition-colors"
            onClick={() => fileInputRef.current?.click()}
            disabled={isSubmitting}
          >
            {t("geminiCli.form.chooseFile")}
          </button>
          <span className="text-sm text-gray-400 truncate max-w-[220px]">
            {selectedFile || t("geminiCli.form.noFile")}
          </span>
        </div>

        <button
          type="submit"
          disabled={isSubmitting}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
            isSubmitting
              ? "bg-indigo-900/50 text-indigo-200 cursor-not-allowed"
              : "bg-indigo-600 hover:bg-indigo-500 text-white"
          }`}
        >
          {isSubmitting
            ? t("geminiCli.form.submitting")
            : t("geminiCli.form.button")}
        </button>
      </div>
    </form>
  );
};

export default CliCredentialUpload;

