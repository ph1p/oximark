/**
 * ironmark — C FFI bindings
 *
 * Compile the Rust crate as a static library first:
 *
 *   cargo build --release
 *
 * Then link against it:
 *
 *   cc -o example example.c -L target/release -l ironmark -lpthread -ldl
 *   (macOS: add -framework CoreFoundation -framework Security)
 *
 * Basic usage:
 *
 *   char *html = ironmark_parse("# Hello\n");
 *   if (html) {
 *       printf("%s\n", html);
 *       ironmark_free(html);
 *   }
 */

#ifndef IRONMARK_H
#define IRONMARK_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * ironmark_parse — parse Markdown and return an HTML string.
 *
 * @param input  Null-terminated UTF-8 Markdown string.
 * @return       Heap-allocated, null-terminated HTML string.
 *               The caller MUST free this pointer with ironmark_free().
 *               Returns NULL if input is NULL or contains invalid UTF-8.
 *
 * Parsing is done with the default ParseOptions:
 *   - hard_breaks          enabled
 *   - enable_highlight     enabled  (==text== → <mark>)
 *   - enable_strikethrough enabled  (~~text~~ → <del>)
 *   - enable_underline     enabled  (++text++ → <u>)
 *   - enable_tables        enabled
 *   - enable_autolink      enabled
 *   - enable_task_lists    enabled
 *   - disable_raw_html     disabled (raw HTML is passed through)
 *   - max_nesting_depth    128
 *   - max_input_size       0 (unlimited)
 *
 * Dangerous URI schemes (javascript:, vbscript:, data: except data:image/…)
 * are always stripped regardless of options.
 */
char *ironmark_parse(const char *input);

/**
 * ironmark_free — free a string returned by ironmark_parse.
 *
 * @param ptr  Pointer returned by ironmark_parse, or NULL (no-op).
 *
 * Passing any other pointer is undefined behaviour.
 */
void ironmark_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif /* IRONMARK_H */
