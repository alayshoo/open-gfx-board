use std::collections::HashMap;
use rusqlite::Connection;

/// Holds runtime variables available for template interpolation.
///
/// Variables are resolved at display time (when a screen/popup is activated),
/// so time-based values reflect the moment of activation.
pub struct TemplateContext {
    pub variables: HashMap<String, String>,
    /// Plugin runtime state: plugin_id -> (key -> string value).
    pub plugin_contexts: HashMap<String, HashMap<String, String>>,
    /// Per-invocation context (e.g. popup trigger context).
    pub popup_context: HashMap<String, String>,
    /// When processing a plugin template, this holds the plugin id.
    pub popup_plugin_id: Option<String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            plugin_contexts: HashMap::new(),
            popup_context: HashMap::new(),
            popup_plugin_id: None,
        }
    }

    /// Build a context from the current runtime state.
    ///
    /// Available built-in variables:
    /// - `program_name`  – name of the currently active program (if any)
    /// - `studio_name`   – name of the active studio (if any)
    /// - `current_time`  – HH:MM:SS at the moment of activation
    /// - `current_date`  – YYYY-MM-DD at the moment of activation
    /// - `current_datetime` – YYYY-MM-DD HH:MM:SS
    pub fn from_runtime(
        program_name: Option<&str>,
        studio_name: Option<&str>,
    ) -> Self {
        let mut vars = HashMap::new();

        if let Some(name) = program_name {
            vars.insert("program_name".to_string(), name.to_string());
        }
        if let Some(name) = studio_name {
            vars.insert("studio_name".to_string(), name.to_string());
        }

        let now = chrono::Local::now();
        vars.insert("current_time".to_string(), now.format("%H:%M:%S").to_string());
        vars.insert("current_date".to_string(), now.format("%Y-%m-%d").to_string());
        vars.insert(
            "current_datetime".to_string(),
            now.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        Self {
            variables: vars,
            plugin_contexts: HashMap::new(),
            popup_context: HashMap::new(),
            popup_plugin_id: None,
        }
    }
}

/// Process an HTML template string, resolving all `{{...}}` expressions.
///
/// # Supported expression types
///
/// - `{{var:name}}`            – Resolves a runtime variable from the context
/// - `{{db:table.column:id}}`  – Looks up a single value from the database
///
/// Unrecognised or failed expressions are left verbatim so the template
/// remains debuggable and can be re-processed later if needed.
pub fn process_template(
    html: &str,
    ctx: &TemplateContext,
    conn: Option<&Connection>,
) -> String {
    let mut result = String::with_capacity(html.len());
    let mut cursor = 0;
    let bytes = html.as_bytes();

    while cursor < bytes.len() {
        // Look for the next `{{`
        if let Some(open) = find_pattern(bytes, cursor, b"{{") {
            // Copy everything before the opening tag
            result.push_str(&html[cursor..open]);

            // Look for the matching `}}`
            if let Some(close) = find_pattern(bytes, open + 2, b"}}") {
                let expr = html[open + 2..close].trim();
                let resolved = resolve_expression(expr, ctx, conn);
                result.push_str(&resolved);
                cursor = close + 2;
            } else {
                // No closing `}}` – copy the `{{` literally and move on
                result.push_str("{{");
                cursor = open + 2;
            }
        } else {
            // No more expressions – copy the rest
            result.push_str(&html[cursor..]);
            break;
        }
    }

    result
}

/// Find the byte offset of `pattern` in `bytes` starting at `from`.
fn find_pattern(bytes: &[u8], from: usize, pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || from + pattern.len() > bytes.len() {
        return None;
    }
    bytes[from..]
        .windows(pattern.len())
        .position(|w| w == pattern)
        .map(|pos| from + pos)
}

/// Resolve a single template expression (the text between `{{` and `}}`).
fn resolve_expression(
    expr: &str,
    ctx: &TemplateContext,
    conn: Option<&Connection>,
) -> String {
    if let Some(var_name) = expr.strip_prefix("var:") {
        // Runtime variable: {{var:program_name}}
        ctx.variables
            .get(var_name.trim())
            .cloned()
            .unwrap_or_else(|| format!("{{{{{}}}}}", expr))
    } else if let Some(db_expr) = expr.strip_prefix("db:") {
        // Database lookup: {{db:programs.name:1}}
        resolve_db_lookup(db_expr.trim(), conn)
            .unwrap_or_else(|| format!("{{{{{}}}}}", expr))
    } else if let Some(plugin_expr) = expr.strip_prefix("plugin:") {
        // Plugin expressions: {{plugin:football:home_score}}
        //                     {{plugin:football:db:teams.name:1}}
        //                     {{plugin:football:context:player_name}}
        resolve_plugin_expression(plugin_expr.trim(), ctx, conn)
            .unwrap_or_else(|| format!("{{{{{}}}}}", expr))
    } else {
        // Unknown expression type – leave as-is
        format!("{{{{{}}}}}", expr)
    }
}

/// Resolve a plugin-namespaced expression.
///
/// Formats:
/// - `plugin_id:state_key`          — plugin runtime state variable
/// - `plugin_id:db:table.column:id` — plugin database lookup
/// - `plugin_id:context:key`        — popup trigger context variable
/// - `plugin_id:asset:path`         — plugin asset URL (resolved to /plugins/{id}/assets/{path})
fn resolve_plugin_expression(
    expr: &str,
    ctx: &TemplateContext,
    conn: Option<&Connection>,
) -> Option<String> {
    // Split on first ':'
    let (plugin_id, rest) = expr.split_once(':')?;
    let plugin_id = plugin_id.trim();

    if let Some(db_expr) = rest.strip_prefix("db:") {
        // Plugin database lookup: {{plugin:football:db:teams.name:1}}
        resolve_plugin_db_lookup(plugin_id, db_expr.trim(), conn)
    } else if let Some(ctx_key) = rest.strip_prefix("context:") {
        // Popup trigger context: {{plugin:football:context:player_name}}
        ctx.popup_context.get(ctx_key.trim()).cloned()
    } else if let Some(asset_path) = rest.strip_prefix("asset:") {
        // Asset URL: {{plugin:football:asset:media/logo.png}}
        Some(format!("/plugins/{}/assets/{}", plugin_id, asset_path.trim()))
    } else {
        // State variable: {{plugin:football:home_score}}
        let key = rest.trim();
        ctx.plugin_contexts
            .get(plugin_id)
            .and_then(|vars| vars.get(key))
            .cloned()
    }
}

/// Resolve a plugin database lookup.
///
/// Format: `table.column:id`
/// The actual table name is `plugin_{plugin_id}_{table}`.
fn resolve_plugin_db_lookup(
    plugin_id: &str,
    expr: &str,
    conn: Option<&Connection>,
) -> Option<String> {
    let conn = conn?;

    let (table_col, id_str) = expr.split_once(':')?;
    let (table, column) = table_col.split_once('.')?;
    let id: i64 = id_str.trim().parse().ok()?;

    // Validate identifiers
    if !crate::plugins::manifest::validate_identifier(table)
        || !crate::plugins::manifest::validate_identifier(column)
        || !crate::plugins::manifest::validate_plugin_id(plugin_id)
    {
        return None;
    }

    let full_table = format!("plugin_{}_{}", plugin_id.replace('-', "_"), table);
    let query = format!("SELECT \"{}\" FROM \"{}\" WHERE id = ?1", column, full_table);
    conn.query_row(&query, [id], |row| row.get::<_, String>(0)).ok()
}

/// Resolve a database lookup expression.
///
/// Format: `table.column:id`
///
/// Only an allowlisted set of table/column pairs is permitted to prevent
/// SQL injection. The id must be a valid integer.
fn resolve_db_lookup(expr: &str, conn: Option<&Connection>) -> Option<String> {
    let conn = conn?;

    // Parse "table.column:id"
    let (table_col, id_str) = expr.split_once(':')?;
    let (table, column) = table_col.split_once('.')?;
    let id: i64 = id_str.trim().parse().ok()?;

    // Allowlist to prevent injection – only known safe table.column pairs
    let allowed = matches!(
        (table, column),
        ("programs", "name")
            | ("programs", "logo_path")
            | ("programs", "bg_path")
            | ("screens", "name")
            | ("screens", "comments")
            | ("screens", "media_type")
            | ("popups", "name")
            | ("popups", "sponsor_name")
            | ("popups", "comments")
            | ("studios", "name")
            | ("studios", "obs_browser_source_address")
    );

    if !allowed {
        return None;
    }

    // Safe to construct – table and column come from the allowlist above
    let query = format!("SELECT \"{}\" FROM \"{}\" WHERE id = ?1", column, table);
    conn.query_row(&query, [id], |row| row.get::<_, String>(0))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_substitution() {
        let mut ctx = TemplateContext::new();
        ctx.variables.insert("name".into(), "World".into());
        let result = process_template("Hello {{var:name}}!", &ctx, None);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_unknown_var_left_intact() {
        let ctx = TemplateContext::new();
        let result = process_template("Hello {{var:missing}}!", &ctx, None);
        assert_eq!(result, "Hello {{var:missing}}!");
    }

    #[test]
    fn test_unknown_expression_type() {
        let ctx = TemplateContext::new();
        let result = process_template("{{foo:bar}}", &ctx, None);
        assert_eq!(result, "{{foo:bar}}");
    }

    #[test]
    fn test_no_expressions() {
        let ctx = TemplateContext::new();
        let html = "<div>No templates here</div>";
        assert_eq!(process_template(html, &ctx, None), html);
    }

    #[test]
    fn test_unclosed_brace() {
        let ctx = TemplateContext::new();
        let result = process_template("Hello {{ world", &ctx, None);
        assert_eq!(result, "Hello {{ world");
    }

    #[test]
    fn test_multiple_vars() {
        let mut ctx = TemplateContext::new();
        ctx.variables.insert("a".into(), "1".into());
        ctx.variables.insert("b".into(), "2".into());
        let result = process_template("{{var:a}} and {{var:b}}", &ctx, None);
        assert_eq!(result, "1 and 2");
    }

    #[test]
    fn test_db_lookup_without_connection() {
        let ctx = TemplateContext::new();
        let result = process_template("{{db:programs.name:1}}", &ctx, None);
        assert_eq!(result, "{{db:programs.name:1}}");
    }
}
