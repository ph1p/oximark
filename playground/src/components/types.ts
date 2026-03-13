export type OutputTab = "preview" | "html" | "ast";
export type MobilePanel = "editor" | OutputTab;

export type ParseOptions = {
  hard_breaks: boolean;
  enable_highlight: boolean;
  enable_strikethrough: boolean;
  enable_underline: boolean;
  enable_tables: boolean;
  enable_autolink: boolean;
  enable_task_lists: boolean;
};

export const DEFAULT_PARSE_OPTIONS: ParseOptions = {
  hard_breaks: true,
  enable_highlight: true,
  enable_strikethrough: true,
  enable_underline: true,
  enable_tables: true,
  enable_autolink: true,
  enable_task_lists: true,
};
