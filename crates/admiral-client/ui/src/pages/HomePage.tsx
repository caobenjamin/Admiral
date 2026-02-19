import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/authStore";
import { useAuth } from "@/hooks/useAuth";

export function HomePage() {
  const user = useAuthStore((s) => s.user);
  const license = useAuthStore((s) => s.license);
  const { loadUserAndLicense } = useAuth();
  const navigate = useNavigate();

  useEffect(() => {
    loadUserAndLicense().then((ok) => {
      if (!ok) navigate("/login");
    });
  }, [loadUserAndLicense, navigate]);

  if (!user) return null;

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-medium">用户信息</h2>
      <div className="rounded border p-4 space-y-1 text-sm">
        <p>邮箱：{user.email}</p>
        <p>姓名：{user.name}</p>
        <p>角色：{user.role}</p>
      </div>
      <h2 className="text-lg font-medium">License</h2>
      <div className="rounded border p-4 space-y-1 text-sm">
        <p>状态：{license?.is_active ? "有效" : "无效"}</p>
        {license?.expires_at && <p>到期：{license.expires_at}</p>}
      </div>
    </div>
  );
}
