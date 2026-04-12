/**
 * Football Editor – Custom Element
 * Tabs: Teams, Players, Starting XI
 */
class FootballEditor extends HTMLElement {
  constructor() {
    super();
    this._sdk = null;
    this._tab = 'teams';
    this._teams = [];
    this._players = [];
    this._formation = [];
    this._selectedTeamId = null;
    this._state = {};
  }

  set sdk(val) {
    this._sdk = val;
    this._init();
  }

  async _init() {
    this._state = await this._sdk.getState();
    await this._loadTeams();
    this._render();
    this._sdk.onDataChanged('teams', () => this._loadTeams().then(() => this._renderContent()));
    this._sdk.onDataChanged('players', () => this._loadPlayers().then(() => this._renderContent()));
    this._sdk.onDataChanged('formation', () => this._loadFormation().then(() => this._renderContent()));
    this._sdk.onStateChanged((s) => { this._state = s; });
  }

  async _loadTeams() {
    try { this._teams = await this._sdk.getData('teams'); } catch { this._teams = []; }
    if (this._teams.length > 0 && !this._selectedTeamId) {
      this._selectedTeamId = this._teams[0].id;
    }
  }

  async _loadPlayers() {
    if (!this._selectedTeamId) { this._players = []; return; }
    try {
      this._players = await this._sdk.queryData('players', { team_id: String(this._selectedTeamId) });
    } catch { this._players = []; }
  }

  async _loadFormation() {
    if (!this._selectedTeamId) { this._formation = []; return; }
    try {
      this._formation = await this._sdk.queryData('formation', { team_id: String(this._selectedTeamId) });
    } catch { this._formation = []; }
  }

  _render() {
    this.innerHTML = `
      <style>
        :host, .fe-root { font-family: inherit; color: var(--text-1, #e4e4e7); }
        .fe-root { display: flex; flex-direction: column; gap: 0; }
        .fe-tabs { display: flex; border-bottom: 1px solid var(--border-1, #3f3f46); margin-bottom: 16px; }
        .fe-tab { padding: 8px 16px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-3, #71717a); cursor: pointer; font-size: 0.85rem; font-weight: 500; font-family: inherit; transition: all 0.15s; }
        .fe-tab:hover { color: var(--text-1, #e4e4e7); }
        .fe-tab.active { color: var(--accent, #38bdf8); border-bottom-color: var(--accent, #38bdf8); }
        .fe-content { min-height: 200px; }
        .fe-section { margin-bottom: 16px; }
        .fe-label { font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-3, #71717a); margin-bottom: 6px; }
        .fe-row { display: flex; gap: 8px; align-items: center; margin-bottom: 6px; flex-wrap: wrap; }
        .fe-input { padding: 6px 10px; background: var(--surface-2, #27272a); border: 1px solid var(--border-1, #3f3f46); border-radius: 6px; color: var(--text-1, #e4e4e7); font-size: 0.85rem; font-family: inherit; }
        .fe-input:focus { outline: none; border-color: var(--accent, #38bdf8); }
        .fe-input-sm { width: 80px; }
        .fe-input-color { width: 50px; height: 32px; padding: 2px; cursor: pointer; }
        .fe-btn { padding: 6px 14px; background: var(--surface-3, #3f3f46); border: 1px solid var(--border-2, #52525b); border-radius: 6px; color: var(--text-1, #e4e4e7); cursor: pointer; font-size: 0.8rem; font-weight: 500; font-family: inherit; transition: background 0.1s; }
        .fe-btn:hover { background: var(--border-1, #3f3f46); }
        .fe-btn-primary { background: var(--accent-dim, rgba(56,189,248,0.15)); border-color: rgba(56,189,248,0.3); color: var(--accent, #38bdf8); }
        .fe-btn-primary:hover { background: rgba(56,189,248,0.25); }
        .fe-btn-danger { background: var(--live-dim, rgba(239,68,68,0.15)); border-color: rgba(239,68,68,0.3); color: #ef4444; }
        .fe-btn-danger:hover { background: rgba(239,68,68,0.25); }
        .fe-table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
        .fe-table th { text-align: left; padding: 6px 10px; background: var(--surface-2, #27272a); border-bottom: 1px solid var(--border-1, #3f3f46); color: var(--text-3, #71717a); font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.04em; }
        .fe-table td { padding: 6px 10px; border-bottom: 1px solid var(--border-1, #3f3f46); }
        .fe-table tr:hover td { background: var(--surface-2, #27272a); }
        .fe-color-dot { width: 16px; height: 16px; border-radius: 50%; display: inline-block; vertical-align: middle; border: 1px solid var(--border-2, #52525b); }
        .fe-select { padding: 6px 10px; background: var(--surface-2, #27272a); border: 1px solid var(--border-1, #3f3f46); border-radius: 6px; color: var(--text-1, #e4e4e7); font-size: 0.85rem; font-family: inherit; }
        .fe-team-selector { margin-bottom: 12px; }
        .fe-formation { display: flex; flex-direction: column; gap: 8px; }
        .fe-form-row { display: flex; align-items: center; gap: 8px; }
        .fe-form-row-label { width: 50px; font-size: 0.75rem; color: var(--text-3, #71717a); font-weight: 600; text-transform: uppercase; flex-shrink: 0; }
        .fe-form-slots { display: flex; gap: 6px; flex-wrap: wrap; }
        .fe-slot-input { width: 50px; text-align: center; }
        .fe-help { font-size: 0.75rem; color: var(--text-3, #71717a); margin: 4px 0; }
      </style>
      <div class="fe-root">
        <div class="fe-tabs">
          <button class="fe-tab ${this._tab === 'teams' ? 'active' : ''}" data-tab="teams">Teams</button>
          <button class="fe-tab ${this._tab === 'players' ? 'active' : ''}" data-tab="players">Players</button>
          <button class="fe-tab ${this._tab === 'formation' ? 'active' : ''}" data-tab="formation">Starting XI</button>
        </div>
        <div class="fe-content" id="fe-content"></div>
      </div>
    `;

    this.querySelectorAll('.fe-tab').forEach(btn => {
      btn.addEventListener('click', () => {
        this._tab = btn.dataset.tab;
        this.querySelectorAll('.fe-tab').forEach(b => b.classList.toggle('active', b.dataset.tab === this._tab));
        this._onTabChange();
      });
    });

    this._renderContent();
  }

  async _onTabChange() {
    if (this._tab === 'players') await this._loadPlayers();
    if (this._tab === 'formation') await this._loadFormation();
    this._renderContent();
  }

  _renderContent() {
    const container = this.querySelector('#fe-content');
    if (!container) return;

    if (this._tab === 'teams') this._renderTeams(container);
    else if (this._tab === 'players') this._renderPlayers(container);
    else if (this._tab === 'formation') this._renderFormation(container);
  }

  // ── Teams Tab ──

  _renderTeams(el) {
    el.innerHTML = `
      <div class="fe-section">
        <div class="fe-label">Add Team</div>
        <div class="fe-row">
          <input class="fe-input" id="fe-team-name" placeholder="Team name" />
          <input class="fe-input fe-input-sm" id="fe-team-short" placeholder="Short" />
          <input class="fe-input fe-input-color" id="fe-team-color1" type="color" value="#ffffff" title="Primary color" />
          <input class="fe-input fe-input-color" id="fe-team-color2" type="color" value="#000000" title="Secondary color" />
          <button class="fe-btn fe-btn-primary" id="fe-team-add">Add</button>
        </div>
      </div>
      <table class="fe-table">
        <thead><tr><th>Name</th><th>Short</th><th>Colors</th><th></th></tr></thead>
        <tbody>
          ${this._teams.map(t => `
            <tr>
              <td>${this._esc(t.name)}</td>
              <td>${this._esc(t.short_name)}</td>
              <td><span class="fe-color-dot" style="background:${t.primary_color}"></span> <span class="fe-color-dot" style="background:${t.secondary_color}"></span></td>
              <td>
                <button class="fe-btn" data-set-team="${t.id}" data-team-name="${this._esc(t.name)}" data-team-color="${t.primary_color}" title="Set as Home">Home</button>
                <button class="fe-btn" data-set-away="${t.id}" data-team-name="${this._esc(t.name)}" data-team-color="${t.primary_color}" title="Set as Away">Away</button>
                <button class="fe-btn fe-btn-danger" data-del-team="${t.id}">Del</button>
              </td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    `;

    el.querySelector('#fe-team-add')?.addEventListener('click', async () => {
      const name = el.querySelector('#fe-team-name')?.value?.trim();
      if (!name) return;
      const short_name = el.querySelector('#fe-team-short')?.value?.trim() || '';
      const primary_color = el.querySelector('#fe-team-color1')?.value || '#ffffff';
      const secondary_color = el.querySelector('#fe-team-color2')?.value || '#000000';
      await this._sdk.insertRow('teams', { name, short_name, primary_color, secondary_color });
      await this._loadTeams();
      this._renderContent();
    });

    el.querySelectorAll('[data-del-team]').forEach(btn => {
      btn.addEventListener('click', async () => {
        await this._sdk.deleteRow('teams', Number(btn.dataset.delTeam));
        await this._loadTeams();
        this._renderContent();
      });
    });

    el.querySelectorAll('[data-set-team]').forEach(btn => {
      btn.addEventListener('click', async () => {
        await this._sdk.setState({
          home_team_id: Number(btn.dataset.setTeam),
          home_team_name: btn.dataset.teamName,
          home_primary_color: btn.dataset.teamColor,
        });
      });
    });

    el.querySelectorAll('[data-set-away]').forEach(btn => {
      btn.addEventListener('click', async () => {
        await this._sdk.setState({
          away_team_id: Number(btn.dataset.setAway),
          away_team_name: btn.dataset.teamName,
          away_primary_color: btn.dataset.teamColor,
        });
      });
    });
  }

  // ── Players Tab ──

  _renderPlayers(el) {
    el.innerHTML = `
      <div class="fe-team-selector">
        <select class="fe-select" id="fe-player-team">
          ${this._teams.map(t => `<option value="${t.id}" ${t.id === this._selectedTeamId ? 'selected' : ''}>${this._esc(t.name)}</option>`).join('')}
        </select>
      </div>
      ${this._teams.length === 0 ? '<p class="fe-help">Create a team first.</p>' : `
        <div class="fe-section">
          <div class="fe-label">Add Player</div>
          <div class="fe-row">
            <input class="fe-input fe-input-sm" id="fe-player-num" type="number" min="1" placeholder="#" />
            <input class="fe-input" id="fe-player-name" placeholder="Name" />
            <button class="fe-btn fe-btn-primary" id="fe-player-add">Add</button>
          </div>
        </div>
        <table class="fe-table">
          <thead><tr><th>#</th><th>Name</th><th></th></tr></thead>
          <tbody>
            ${this._players.map(p => `
              <tr>
                <td>${p.number}</td>
                <td>${this._esc(p.name)}</td>
                <td><button class="fe-btn fe-btn-danger" data-del-player="${p.id}">Del</button></td>
              </tr>
            `).join('')}
          </tbody>
        </table>
      `}
    `;

    el.querySelector('#fe-player-team')?.addEventListener('change', async (e) => {
      this._selectedTeamId = Number(e.target.value);
      await this._loadPlayers();
      this._renderContent();
    });

    el.querySelector('#fe-player-add')?.addEventListener('click', async () => {
      const number = Number(el.querySelector('#fe-player-num')?.value);
      const name = el.querySelector('#fe-player-name')?.value?.trim();
      if (!number || !name) return;
      await this._sdk.insertRow('players', { team_id: this._selectedTeamId, number, name });
      await this._loadPlayers();
      this._renderContent();
    });

    el.querySelectorAll('[data-del-player]').forEach(btn => {
      btn.addEventListener('click', async () => {
        await this._sdk.deleteRow('players', Number(btn.dataset.delPlayer));
        await this._loadPlayers();
        this._renderContent();
      });
    });
  }

  // ── Formation Tab ──

  _renderFormation(el) {
    const ROW_LABELS = ['GK', 'DEF', 'MID', 'MID', 'FWD'];

    // Build current formation state from DB data
    const rows = [1, 0, 0, 0, 0]; // default: GK only
    const slots = [[], [], [], [], []];
    for (const f of this._formation) {
      const ri = f.row_index;
      if (ri >= 0 && ri < 5) {
        while (slots[ri].length <= f.slot_index) slots[ri].push(0);
        slots[ri][f.slot_index] = f.player_number;
        if (slots[ri].length > rows[ri]) rows[ri] = slots[ri].length;
      }
    }
    // Ensure row 0 is always 1
    rows[0] = 1;
    if (slots[0].length === 0) slots[0] = [0];

    el.innerHTML = `
      <div class="fe-team-selector">
        <select class="fe-select" id="fe-form-team">
          ${this._teams.map(t => `<option value="${t.id}" ${t.id === this._selectedTeamId ? 'selected' : ''}>${this._esc(t.name)}</option>`).join('')}
        </select>
      </div>
      ${this._teams.length === 0 ? '<p class="fe-help">Create a team first.</p>' : `
        <p class="fe-help">Set the number of players in each row and enter their shirt numbers. Row 1 (GK) is always 1 player.</p>
        <div class="fe-formation">
          ${ROW_LABELS.map((label, ri) => `
            <div class="fe-form-row">
              <span class="fe-form-row-label">${label}</span>
              ${ri === 0 ? '' : `<select class="fe-select fe-row-count" data-row="${ri}" style="width:50px">
                ${[0,1,2,3,4,5,6].map(n => `<option value="${n}" ${n === rows[ri] ? 'selected' : ''}>${n}</option>`).join('')}
              </select>`}
              <div class="fe-form-slots" data-slots="${ri}">
                ${Array.from({length: rows[ri]}, (_, si) => `
                  <input class="fe-input fe-slot-input" type="number" min="1"
                    data-row="${ri}" data-slot="${si}"
                    value="${slots[ri][si] || ''}"
                    placeholder="#" />
                `).join('')}
              </div>
            </div>
          `).join('')}
        </div>
        <div class="fe-row" style="margin-top:12px;">
          <button class="fe-btn fe-btn-primary" id="fe-form-save">Save Formation</button>
        </div>
      `}
    `;

    el.querySelector('#fe-form-team')?.addEventListener('change', async (e) => {
      this._selectedTeamId = Number(e.target.value);
      await this._loadFormation();
      this._renderContent();
    });

    // Row count changes: re-render slots
    el.querySelectorAll('.fe-row-count').forEach(sel => {
      sel.addEventListener('change', () => {
        const ri = Number(sel.dataset.row);
        const count = Number(sel.value);
        const container = el.querySelector(`[data-slots="${ri}"]`);
        if (!container) return;
        container.innerHTML = Array.from({length: count}, (_, si) => `
          <input class="fe-input fe-slot-input" type="number" min="1"
            data-row="${ri}" data-slot="${si}"
            value=""
            placeholder="#" />
        `).join('');
      });
    });

    el.querySelector('#fe-form-save')?.addEventListener('click', async () => {
      if (!this._selectedTeamId) return;

      // Collect all slot values
      const entries = [];
      el.querySelectorAll('.fe-slot-input').forEach(input => {
        const num = Number(input.value);
        if (!num) return;
        entries.push({
          team_id: this._selectedTeamId,
          row_index: Number(input.dataset.row),
          slot_index: Number(input.dataset.slot),
          player_number: num,
        });
      });

      // Delete existing formation for this team
      for (const f of this._formation) {
        await this._sdk.deleteRow('formation', f.id);
      }

      // Insert new entries
      for (const entry of entries) {
        await this._sdk.insertRow('formation', entry);
      }

      await this._loadFormation();
      this._renderContent();
    });
  }

  _esc(str) {
    const div = document.createElement('div');
    div.textContent = String(str ?? '');
    return div.innerHTML;
  }
}

customElements.define('football-editor', FootballEditor);
export default FootballEditor;
