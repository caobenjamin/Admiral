import { Link } from "react-router-dom";

export function Sidebar() {
  return (
    <aside className="w-48 border-r bg-gray-50 p-2 flex flex-col gap-1">
      <Link to="/" className="px-3 py-2 rounded hover:bg-gray-200">
        首页
      </Link>
      <Link to="/settings/platforms" className="px-3 py-2 rounded hover:bg-gray-200">
        B 站授权
      </Link>
      <Link to="/settings/mapi" className="px-3 py-2 rounded hover:bg-gray-200">
        MAPI 调试
      </Link>
      <Link to="/settings/llm" className="px-3 py-2 rounded hover:bg-gray-200">
        LLM 对话
      </Link>
    </aside>
  );
}
