import { Header } from "./Header";
import { LeftTab } from "./LeftTab";
import { MainContent } from "./MainContent";
import { StatusBar } from "./StatusBar";
import { Toast } from "@/components/common/Toast";

export function AppShell() {
  return (
    <>
      <div className="admiral-app">
        <Header />
        <div
          style={{
            display: "flex",
            flexDirection: "row",
            flex: 1,
            minHeight: 0,
            minWidth: 0,
            overflow: "hidden",
          }}
        >
          <LeftTab />
          <MainContent />
        </div>
        <StatusBar />
      </div>
      <Toast />
    </>
  );
}
