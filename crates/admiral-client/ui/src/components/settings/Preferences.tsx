import { useEffect, useState } from "react";
import * as tauri from "@/lib/tauri";
import type { LlmModelInfo } from "@/lib/types";

const PROVIDER_OPTIONS = [
  { value: "deepseek", label: "DeepSeek" },
  { value: "openai", label: "OpenAI GPT" },
];

function sortModels(models: LlmModelInfo[]) {
  return [...models].sort((a, b) => a.id.localeCompare(b.id));
}

export function Preferences() {
  const [provider, setProvider] = useState("deepseek");
  const [model, setModel] = useState("deepseek-chat");
  const [models, setModels] = useState<LlmModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState("");

  const loadModels = async (nextProvider: string, preferredModel?: string) => {
    setLoading(true);
    setMessage("");
    try {
      const response = await tauri.listLlmModels(nextProvider);
      const nextModels = sortModels(response.models);
      setModels(nextModels);
      const targetModel =
        preferredModel && nextModels.some((item) => item.id === preferredModel)
          ? preferredModel
          : nextModels[0]?.id ?? "";
      setModel(targetModel);
    } catch (error) {
      setModels([]);
      setModel("");
      setMessage(`模型列表获取失败: ${String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;
    (async () => {
      try {
        const selection = await tauri.getLlmSelection();
        if (!active) return;
        setProvider(selection.provider);
        setModel(selection.model);
        await loadModels(selection.provider, selection.model);
      } catch (error) {
        if (!active) return;
        setMessage(`读取当前配置失败: ${String(error)}`);
        setLoading(false);
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  const handleProviderChange = async (nextProvider: string) => {
    setProvider(nextProvider);
    await loadModels(nextProvider);
  };

  const handleSave = async () => {
    if (!provider || !model) {
      setMessage("请先选择 provider 和模型。");
      return;
    }
    setSaving(true);
    setMessage("");
    try {
      await tauri.setLlmSelection(provider, model);
      setMessage("全局模型配置已保存。");
    } catch (error) {
      setMessage(`保存失败: ${String(error)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="admiral-grid-page">
      <div className="settings-panel" style={{ maxWidth: 880 }}>
        <h2 className="settings-title">偏好设置</h2>
        <p className="settings-subtitle">全局 LLM 配置会应用到代理对话、规格提取、记忆更新和联调页面。</p>

        <div style={{ marginTop: 14, display: "grid", gap: 12 }}>
          <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, padding: 16, background: "#ffffff" }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)" }}>全局大模型</div>
            <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>
              选择 OpenAI 时，页面会通过服务端调用 OpenAI 官方模型列表接口实时刷新可用模型。
            </p>

            <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12, marginTop: 14 }}>
              <label style={{ display: "grid", gap: 6 }}>
                <span style={{ fontSize: 12, fontWeight: 700, color: "var(--admiral-text-secondary)" }}>Provider</span>
                <select
                  value={provider}
                  onChange={(e) => void handleProviderChange(e.target.value)}
                  className="admiral-field"
                  style={{ height: 40, padding: "0 10px" }}
                >
                  {PROVIDER_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label style={{ display: "grid", gap: 6 }}>
                <span style={{ fontSize: 12, fontWeight: 700, color: "var(--admiral-text-secondary)" }}>Model</span>
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="admiral-field"
                  style={{ height: 40, padding: "0 10px" }}
                  disabled={loading || models.length === 0}
                >
                  {models.length === 0 ? (
                    <option value="">暂无可用模型</option>
                  ) : (
                    models.map((item) => (
                      <option key={item.id} value={item.id}>
                        {item.id}
                      </option>
                    ))
                  )}
                </select>
              </label>
            </div>

            <div style={{ display: "flex", gap: 8, marginTop: 14, flexWrap: "wrap" }}>
              <button type="button" className="btn btn-secondary" onClick={() => void loadModels(provider, model)} disabled={loading}>
                {loading ? "刷新中" : "刷新模型列表"}
              </button>
              <button type="button" className="btn btn-primary" onClick={() => void handleSave()} disabled={saving || loading || !model}>
                {saving ? "保存中" : "保存全局配置"}
              </button>
            </div>

            <div style={{ marginTop: 12, fontSize: 12, color: "var(--admiral-text-secondary)" }}>
              当前拉取数量: {models.length}
            </div>
            {message ? (
              <div style={{ marginTop: 10, fontSize: 13, color: message.includes("失败") ? "#c62828" : "var(--admiral-accent-700)" }}>
                {message}
              </div>
            ) : null}
          </div>

          <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, padding: 14, background: "#ffffff" }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)" }}>界面主题</div>
            <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>
              当前版本默认使用浅色主题，后续支持自定义主题方案。
            </p>
          </div>

          <div style={{ border: "1px solid var(--admiral-border)", borderRadius: 12, padding: 14, background: "#ffffff" }}>
            <div style={{ fontSize: 14, fontWeight: 700, color: "var(--admiral-text-primary)" }}>语言与区域</div>
            <p style={{ margin: "6px 0 0", fontSize: 13, color: "var(--admiral-text-secondary)" }}>
              当前版本默认中文，后续将支持多语言切换与时间格式配置。
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
