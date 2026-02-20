// PRD 12.4: 任务创建流程（模板选择或自然语言输入）
// 占位：后续接 Agent 交互与 dry-run

import { useState } from "react";
import { Link } from "react-router-dom";

type Step = "name" | "type" | "confirm";

export function TaskCreate() {
  const [step, setStep] = useState<Step>("name");
  const [name, setName] = useState("");
  const [type, setType] = useState<"instant" | "scheduled">("instant");

  return (
    <div className="p-4">
      <div className="mb-4">
        <Link to="/" className="text-sm text-blue-600 hover:underline">← 返回首页</Link>
      </div>
      <div className="max-w-md rounded-lg border border-gray-200 bg-white p-6 space-y-4">
        <h2 className="text-lg font-semibold">新建任务</h2>
      {step === "name" && (
        <>
          <label className="block text-sm font-medium text-gray-700">
            任务名称
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="例如：批量创建 B 站广告计划"
            className="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm"
          />
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setStep("type")}
              className="px-4 py-2 rounded-lg bg-blue-600 text-white text-sm font-medium"
            >
              下一步
            </button>
          </div>
        </>
      )}
      {step === "type" && (
        <>
          <p className="text-sm text-gray-600">选择任务类型</p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setType("instant")}
              className={`px-4 py-2 rounded-lg border text-sm ${
                type === "instant" ? "border-blue-600 bg-blue-50 text-blue-700" : "border-gray-300"
              }`}
            >
              立即执行
            </button>
            <button
              type="button"
              onClick={() => setType("scheduled")}
              className={`px-4 py-2 rounded-lg border text-sm ${
                type === "scheduled" ? "border-blue-600 bg-blue-50 text-blue-700" : "border-gray-300"
              }`}
            >
              定时任务
            </button>
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setStep("name")}
              className="px-4 py-2 rounded-lg border border-gray-300 text-sm"
            >
              上一步
            </button>
            <button
              type="button"
              onClick={() => setStep("confirm")}
              className="px-4 py-2 rounded-lg bg-blue-600 text-white text-sm font-medium"
            >
              下一步
            </button>
          </div>
        </>
      )}
      {step === "confirm" && (
        <>
          <p className="text-sm text-gray-600">
            任务「{name}」将创建为{type === "instant" ? "立即执行" : "定时"}任务。后续可在对话中完善并 dry-run。
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setStep("type")}
              className="px-4 py-2 rounded-lg border border-gray-300 text-sm"
            >
              上一步
            </button>
            <button
              type="button"
              className="px-4 py-2 rounded-lg bg-blue-600 text-white text-sm font-medium"
            >
              创建（占位）
            </button>
          </div>
        </>
      )}
      </div>
    </div>
  );
}
