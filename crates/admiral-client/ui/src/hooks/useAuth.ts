import { useCallback } from "react";
import { useNavigate } from "react-router-dom";
import * as tauri from "@/lib/tauri";
import { useAuthStore } from "@/stores/authStore";

export function useAuth() {
  const navigate = useNavigate();
  const { setUser, setLicense } = useAuthStore();

  const login = useCallback(
    async (email: string, password: string) => {
      await tauri.authLogin({ email, password });
      const user = await tauri.authMe();
      const license = await tauri.authLicense();
      setUser(user);
      setLicense(license);
      navigate("/");
    },
    [navigate, setUser, setLicense]
  );

  const register = useCallback(
    async (email: string, password: string, name: string, company: string, inviteCode: string) => {
      await tauri.authRegister({ email, password, name, company: company || undefined, invite_code: inviteCode });
      const user = await tauri.authMe();
      const license = await tauri.authLicense();
      setUser(user);
      setLicense(license);
      navigate("/");
    },
    [navigate, setUser, setLicense]
  );

  const logout = useCallback(async () => {
    await tauri.authLogout();
    setUser(null);
    setLicense(null);
    navigate("/login");
  }, [navigate, setUser, setLicense]);

  const loadUserAndLicense = useCallback(async () => {
    try {
      const [user, license] = await Promise.all([tauri.authMe(), tauri.authLicense()]);
      setUser(user);
      setLicense(license);
      return true;
    } catch {
      setUser(null);
      setLicense(null);
      return false;
    }
  }, [setUser, setLicense]);

  return { login, register, logout, loadUserAndLicense };
}
