import type { RefObject } from "react";
import { OutputTabs } from "./OutputTabs";
import { OptionsPanel } from "./OptionsPanel";
import type { MobilePanel, OutputTab, ParseOptions } from "./types";
import { MarkdownEditor } from "./MarkdownEditor";
import type { EditorView } from "@codemirror/view";

type SplitPanelsProps = {
  mobilePanel: MobilePanel;
  outputTab: OutputTab;
  onOutputTabChange: (tab: OutputTab) => void;
  markdown: string;
  onMarkdownValueChange: (value: string) => void;
  onMarkdownDocChange: (value: string) => void;
  onEditorReady: (view: EditorView) => void;
  options: ParseOptions;
  onOptionsChange: (options: ParseOptions) => void;
  previewRef: RefObject<HTMLDivElement | null>;
  htmlSourceContainerRef: RefObject<HTMLDivElement | null>;
  astSourceContainerRef: RefObject<HTMLDivElement | null>;
};

export function SplitPanels({
  mobilePanel,
  outputTab,
  onOutputTabChange,
  markdown,
  onMarkdownValueChange,
  onMarkdownDocChange,
  onEditorReady,
  options,
  onOptionsChange,
  previewRef,
  htmlSourceContainerRef,
  astSourceContainerRef,
}: SplitPanelsProps) {
  const showEditor = mobilePanel === "editor";

  return (
    <div id="main-split" className="flex flex-col md:flex-row flex-1 min-h-0">
      <div
        id="panel-editor"
        className={`${showEditor ? "flex" : "hidden"} md:flex flex-1 md:flex-[0_0_50%] md:min-w-0 flex-col md:border-r border-zinc-200 dark:border-zinc-800 min-h-0`}
      >
        <div className="hidden md:block px-4 pt-2.5 pb-2 text-xs font-medium text-zinc-400 uppercase tracking-wider border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50">
          Markdown
        </div>
        <OptionsPanel options={options} onChange={onOptionsChange} />
        <div className="flex-1 min-h-0 overflow-hidden">
          <MarkdownEditor
            value={markdown}
            onValueChange={onMarkdownValueChange}
            onDocChange={onMarkdownDocChange}
            onEditorReady={onEditorReady}
          />
        </div>
      </div>
      <div
        id="panel-output"
        className={`${showEditor ? "hidden" : "flex"} md:flex md:flex-[0_0_50%] md:min-w-0 flex-1 flex-col min-h-0`}
      >
        <OutputTabs outputTab={outputTab} onChange={onOutputTabChange} />
        <div
          id="panel-preview"
          role="tabpanel"
          aria-labelledby="tab-preview"
          className={`${outputTab === "preview" ? "flex" : "hidden"} flex-1 min-h-0 overflow-auto p-3 md:p-5`}
        >
          <div ref={previewRef} className="prose" />
        </div>
        <div
          id="panel-html"
          role="tabpanel"
          aria-labelledby="tab-html"
          className={`${outputTab === "html" ? "block" : "hidden"} flex-1 min-h-0 overflow-hidden`}
        >
          <div ref={htmlSourceContainerRef} className="h-full" />
        </div>
        <div
          id="panel-ast"
          role="tabpanel"
          aria-labelledby="tab-ast"
          className={`${outputTab === "ast" ? "block" : "hidden"} flex-1 min-h-0 overflow-hidden`}
        >
          <div ref={astSourceContainerRef} className="h-full" />
        </div>
      </div>
    </div>
  );
}
