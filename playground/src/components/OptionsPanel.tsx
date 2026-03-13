import { useState } from "react";
import type { ParseOptions } from "./types";

type OptionsPanelProps = {
  options: ParseOptions;
  onChange: (options: ParseOptions) => void;
};

const OPTION_LABELS: { key: keyof ParseOptions; label: string; description: string }[] = [
  { key: "hard_breaks", label: "Hard breaks", description: "Newlines become <br />" },
  { key: "enable_highlight", label: "Highlight", description: "==text== → <mark>" },
  { key: "enable_strikethrough", label: "Strikethrough", description: "~~text~~ → <del>" },
  { key: "enable_underline", label: "Underline", description: "++text++ → <u>" },
  { key: "enable_tables", label: "Tables", description: "Pipe table syntax" },
  { key: "enable_autolink", label: "Autolink", description: "Bare URLs and emails" },
  { key: "enable_task_lists", label: "Task lists", description: "- [x] checkbox items" },
];

export function OptionsPanel({ options, onChange }: OptionsPanelProps) {
  const [open, setOpen] = useState(false);

  const toggle = (key: keyof ParseOptions) => {
    onChange({ ...options, [key]: !options[key] });
  };

  return (
    <div className="border-b border-zinc-200 dark:border-zinc-800">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-4 py-2 text-xs font-medium text-zinc-400 uppercase tracking-wider bg-zinc-50 dark:bg-zinc-900/50 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors cursor-pointer"
      >
        <span>Options</span>
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={`transition-transform ${open ? "rotate-180" : ""}`}
        >
          <path d="M3 4.5L6 7.5L9 4.5" />
        </svg>
      </button>
      {open && (
        <div className="px-4 py-2.5 bg-zinc-50/50 dark:bg-zinc-900/30 grid grid-cols-2 gap-x-4 gap-y-1">
          {OPTION_LABELS.map(({ key, label, description }) => (
            <label
              key={key}
              className="flex items-center gap-2 py-1 cursor-pointer group"
              title={description}
            >
              <button
                type="button"
                role="switch"
                aria-checked={options[key]}
                onClick={() => toggle(key)}
                className={`relative shrink-0 w-7 h-4 rounded-full transition-colors ${
                  options[key] ? "bg-zinc-900 dark:bg-zinc-100" : "bg-zinc-300 dark:bg-zinc-700"
                }`}
              >
                <span
                  className={`absolute top-0.5 left-0.5 w-3 h-3 rounded-full transition-transform ${
                    options[key]
                      ? "translate-x-3 bg-white dark:bg-zinc-900"
                      : "translate-x-0 bg-white dark:bg-zinc-400"
                  }`}
                />
              </button>
              <span className="text-xs text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors select-none">
                {label}
              </span>
            </label>
          ))}
        </div>
      )}
    </div>
  );
}
