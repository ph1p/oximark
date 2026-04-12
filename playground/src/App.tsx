import { useEffect, useRef, useState } from "react";
import type { EditorView } from "@codemirror/view";
import { Header } from "./components/Header";
import { MobileTabs } from "./components/MobileTabs";
import { SplitPanels } from "./components/SplitPanels";
import { BenchmarkPage } from "./components/BenchmarkPage";
import { AstToMarkdownPage } from "./components/AstToMarkdownPage";
import { HtmlToMarkdownPage } from "./components/HtmlToMarkdownPage";
import { DEFAULT_MARKDOWN, initPlayground, type PlaygroundController } from "./playground";
import type { AppView, MobilePanel, OutputTab, ParseOptions } from "./components/types";
import { DEFAULT_PARSE_OPTIONS } from "./components/types";

const MARKDOWN_STORAGE_KEY = "playground:markdown";
const OUTPUT_TAB_STORAGE_KEY = "playground:output-tab";
const MOBILE_PANEL_STORAGE_KEY = "playground:mobile-panel";
const OPTIONS_STORAGE_KEY = "playground:options";

function readOutputTab(): OutputTab {
  const value = localStorage.getItem(OUTPUT_TAB_STORAGE_KEY);
  return value === "html" || value === "ast" ? value : "preview";
}

function readMobilePanel(): MobilePanel {
  const value = localStorage.getItem(MOBILE_PANEL_STORAGE_KEY);
  return value === "preview" || value === "html" || value === "ast" ? value : "editor";
}

function readOptions(): ParseOptions {
  try {
    const raw = localStorage.getItem(OPTIONS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...DEFAULT_PARSE_OPTIONS, ...parsed };
    }
  } catch {
    /* ignore */
  }
  return { ...DEFAULT_PARSE_OPTIONS };
}

function readAppView(): AppView {
  const hash = location.hash;
  if (hash === "#benchmark") return "benchmarks";
  if (hash === "#ast-to-md") return "ast-to-md";
  if (hash === "#html-to-md") return "html-to-md";
  return "playground";
}

export function App() {
  const [appView, setAppView] = useState<AppView>(readAppView);

  useEffect(() => {
    const onHashChange = () => setAppView(readAppView());
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);
  const [statusText, setStatusText] = useState("loading wasm...");
  const [outputTab, setOutputTab] = useState<OutputTab>(() => readOutputTab());
  const [mobilePanel, setMobilePanel] = useState<MobilePanel>(() => readMobilePanel());
  const [markdown, setMarkdown] = useState(
    () => localStorage.getItem(MARKDOWN_STORAGE_KEY) ?? DEFAULT_MARKDOWN,
  );
  const [options, setOptions] = useState<ParseOptions>(readOptions);
  const outputTabRef = useRef(outputTab);
  const controllerRef = useRef<PlaygroundController | null>(null);
  const pendingEditorRef = useRef<EditorView | null>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const htmlSourceContainerRef = useRef<HTMLDivElement>(null);
  const astSourceContainerRef = useRef<HTMLDivElement>(null);

  outputTabRef.current = outputTab;

  useEffect(() => {
    if (appView !== "playground") return;

    let cancelled = false;

    const preview = previewRef.current;
    const htmlSourceContainer = htmlSourceContainerRef.current;
    const astSourceContainer = astSourceContainerRef.current;
    if (!preview || !htmlSourceContainer || !astSourceContainer) {
      return;
    }

    void initPlayground({
      preview,
      htmlSourceContainer,
      astSourceContainer,
      getOutputTab: () => outputTabRef.current,
      onStatusChange: setStatusText,
    })
      .then((controller) => {
        if (cancelled) {
          controller.dispose();
          return;
        }
        controllerRef.current = controller;
        controller.setOutputTab(outputTabRef.current);
        controller.updateOptions(options);
        controller.updateMarkdown(markdown);
        if (pendingEditorRef.current) {
          controller.attachEditorView(pendingEditorRef.current);
        }
      })
      .catch((err) => {
        console.error("initPlayground failed:", err);
        if (!cancelled) {
          setStatusText("failed to load wasm");
        }
      });

    return () => {
      cancelled = true;
      controllerRef.current?.dispose();
      controllerRef.current = null;
    };
  }, [appView]);

  useEffect(() => {
    controllerRef.current?.setOutputTab(outputTab);
  }, [outputTab]);

  useEffect(() => {
    localStorage.setItem(MARKDOWN_STORAGE_KEY, markdown);
  }, [markdown]);

  useEffect(() => {
    localStorage.setItem(OUTPUT_TAB_STORAGE_KEY, outputTab);
  }, [outputTab]);

  useEffect(() => {
    localStorage.setItem(MOBILE_PANEL_STORAGE_KEY, mobilePanel);
  }, [mobilePanel]);

  useEffect(() => {
    localStorage.setItem(OPTIONS_STORAGE_KEY, JSON.stringify(options));
    controllerRef.current?.updateOptions(options);
  }, [options]);

  const onMobilePanelChange = (panel: MobilePanel) => {
    setMobilePanel(panel);
    if (panel !== "editor") {
      setOutputTab(panel);
    }
  };

  const onEditorReady = (view: EditorView) => {
    pendingEditorRef.current = view;
    controllerRef.current?.attachEditorView(view);
  };

  const onMarkdownDocChange = (value: string) => {
    controllerRef.current?.updateMarkdown(value);
  };

  const renderContent = () => {
    switch (appView) {
      case "benchmarks":
        return <BenchmarkPage />;
      case "ast-to-md":
        return <AstToMarkdownPage />;
      case "html-to-md":
        return <HtmlToMarkdownPage />;
      default:
        return (
          <>
            <MobileTabs panel={mobilePanel} onChange={onMobilePanelChange} />
            <SplitPanels
              mobilePanel={mobilePanel}
              outputTab={outputTab}
              onOutputTabChange={setOutputTab}
              markdown={markdown}
              onMarkdownValueChange={setMarkdown}
              onMarkdownDocChange={onMarkdownDocChange}
              onEditorReady={onEditorReady}
              options={options}
              onOptionsChange={setOptions}
              previewRef={previewRef}
              htmlSourceContainerRef={htmlSourceContainerRef}
              astSourceContainerRef={astSourceContainerRef}
              getHtml={() => controllerRef.current?.getHtml() ?? ""}
              getAst={() => controllerRef.current?.getAst() ?? ""}
            />
          </>
        );
    }
  };

  return (
    <div className="h-full flex flex-col bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 transition-colors">
      <Header statusText={statusText} currentView={appView} />
      {renderContent()}
    </div>
  );
}
