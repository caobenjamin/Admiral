import { Link } from "react-router-dom";

export function SetupPage() {
  return (
    <div className="p-6 max-w-xl">
      <h2 className="text-lg font-semibold mb-4" style={{ color: "var(--admiral-text-primary)" }}>
        初始化向导
      </h2>
      <p className="text-sm mb-4" style={{ color: "var(--admiral-text-secondary)" }}>
        完成权限申请与平台授权后即可开始使用。您可以在设置中随时管理。
      </p>
      <Link
        to="/settings/platforms"
        className="inline-flex px-4 py-2 text-sm font-medium text-white rounded-md bg-primary"
        style={{ borderRadius: "var(--admiral-radius-button)" }}
      >
        前往平台授权
      </Link>
    </div>
  );
}
