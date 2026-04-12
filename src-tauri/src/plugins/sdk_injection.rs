/// Auto-injection of a lightweight Plugin SDK into plugin HTML templates.
///
/// When a plugin screen or popup HTML is sent to the OBS browser-source page,
/// this module appends a `<script>` block that provides `window.__pluginSDK`.
///
/// The SDK is entirely `fetch`-based (no Socket.IO dependency inside the
/// iframe). For live overlays (screens) it polls the server for state changes
/// every 300 ms. For short-lived popups it only exposes one-shot fetch methods.

/// Generate the full `<script>…</script>` block to inject into plugin HTML.
///
/// * `plugin_id`  – the plugin slug, e.g. `"football"`
/// * `live`       – when `true` the SDK auto-starts a 300 ms polling loop that
///                  dispatches `plugin-state` events and updates elements with
///                  `data-plugin-var` attributes.  Set this to `true` for screens
///                  (persistent overlays) and `false` for popups (ephemeral).
pub fn sdk_script(plugin_id: &str, live: bool) -> String {
    // The JavaScript below is a self-contained IIFE that:
    //   1. Exposes fetch-based helpers on `window.__pluginSDK`
    //   2. Optionally polls `/plugins/{id}/state` and auto-updates the DOM
    format!(
        r#"<script data-plugin-sdk>
(function() {{
  var PLUGIN_ID = {plugin_id_json};
  var BASE = '/plugins/' + PLUGIN_ID;
  var _state = {{}};
  var _pollTimer = null;

  function fetchJson(url, opts) {{
    return fetch(url, opts || {{}}).then(function(r) {{
      if (!r.ok) throw new Error(r.statusText);
      return r.json();
    }});
  }}

  var sdk = {{
    pluginId: PLUGIN_ID,

    getState: function() {{
      return fetchJson(BASE + '/state').then(function(s) {{ _state = s; return s; }});
    }},

    setState: function(updates) {{
      return fetchJson(BASE + '/state', {{
        method: 'PUT',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify(updates)
      }}).then(function(s) {{ _state = s; return s; }});
    }},

    getData: function(table) {{
      return fetchJson(BASE + '/data/' + encodeURIComponent(table));
    }},

    getRow: function(table, id) {{
      return fetchJson(BASE + '/data/' + encodeURIComponent(table) + '/' + id);
    }},

    queryData: function(table, filters) {{
      var params = new URLSearchParams(filters || {{}});
      return fetchJson(BASE + '/data/' + encodeURIComponent(table) + '/query?' + params);
    }},

    insertRow: function(table, data) {{
      return fetchJson(BASE + '/data/' + encodeURIComponent(table), {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify(data)
      }});
    }},

    updateRow: function(table, id, data) {{
      return fetchJson(BASE + '/data/' + encodeURIComponent(table) + '/' + id, {{
        method: 'PUT',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify(data)
      }});
    }},

    deleteRow: function(table, id) {{
      return fetchJson(BASE + '/data/' + encodeURIComponent(table) + '/' + id, {{
        method: 'DELETE'
      }});
    }},

    triggerPopup: function(templateId, context, duration) {{
      return fetchJson(BASE + '/trigger-popup', {{
        method: 'POST',
        headers: {{'Content-Type': 'application/json'}},
        body: JSON.stringify({{template_id: templateId, context: context || {{}}, duration: duration}})
      }});
    }},

    assetUrl: function(path) {{
      return BASE + '/assets/' + path;
    }},

    startPolling: function() {{
      if (_pollTimer) return;
      _pollTimer = setInterval(pollState, 300);
    }},

    stopPolling: function() {{
      if (_pollTimer) {{ clearInterval(_pollTimer); _pollTimer = null; }}
    }}
  }};

  function applyAutoVars(state) {{
    var els = document.querySelectorAll('[data-plugin-var]');
    for (var i = 0; i < els.length; i++) {{
      var key = els[i].getAttribute('data-plugin-var');
      if (key in state) {{
        var val = state[key];
        els[i].textContent = (val === null || val === undefined) ? '' : String(val);
      }}
    }}
  }}

  function pollState() {{
    fetchJson(BASE + '/state').then(function(newState) {{
      var prev = JSON.stringify(_state);
      var next = JSON.stringify(newState);
      _state = newState;
      if (prev !== next) {{
        applyAutoVars(newState);
        window.dispatchEvent(new CustomEvent('plugin-state', {{detail: newState}}));
      }}
    }}).catch(function() {{}});
  }}

  window.__pluginSDK = sdk;

  {live_init}
}})();
</script>"#,
        plugin_id_json = serde_json::to_string(plugin_id).unwrap_or_else(|_| format!("\"{}\"", plugin_id)),
        live_init = if live {
            "// Auto-start polling for live overlays\n  sdk.startPolling();\n  sdk.getState().then(function(s) { applyAutoVars(s); window.dispatchEvent(new CustomEvent('plugin-state', {detail: s})); }).catch(function() {});"
        } else {
            "// Popup mode – no auto-polling"
        },
    )
}

/// Inject the plugin SDK script into an HTML string.
///
/// If the HTML contains a `</body>` tag the script is inserted just before it.
/// Otherwise it is appended at the very end.
pub fn inject_sdk(html: &str, plugin_id: &str, live: bool) -> String {
    let script = sdk_script(plugin_id, live);

    if let Some(pos) = html.to_ascii_lowercase().rfind("</body>") {
        let mut out = String::with_capacity(html.len() + script.len() + 1);
        out.push_str(&html[..pos]);
        out.push('\n');
        out.push_str(&script);
        out.push('\n');
        out.push_str(&html[pos..]);
        out
    } else {
        let mut out = String::with_capacity(html.len() + script.len() + 1);
        out.push_str(html);
        out.push('\n');
        out.push_str(&script);
        out
    }
}
