import { Header } from "./Header";
import { LeftTab } from "./LeftTab";
import { MainContent } from "./MainContent";
import { StatusBar } from "./StatusBar";

/** 整体布局：Header | LeftTab + MainContent | StatusBar，关键样式内联保证渲染稳定 */
export function AppShell() {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        minHeight: 0,
        overflow: "hidden",
        backgroundColor: "var(--admiral-bg)",
      }}
    >
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
  );
}
