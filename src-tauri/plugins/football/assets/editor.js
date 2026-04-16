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
    this._formationDirty = false;
    this._formationPendingAction = null;
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
    this._sdk.onStateChanged((s) => { this._state = s; this._renderContent(); });
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
      this._players.sort((a, b) => a.number - b.number);
    } catch { this._players = []; }
  }

  async _loadFormation() {
    if (!this._selectedTeamId) { this._formation = []; this._formationDirty = false; return; }
    try {
      this._formation = await this._sdk.queryData('formation', { team_id: String(this._selectedTeamId) });
    } catch { this._formation = []; }
    this._formationDirty = false;
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
        .fe-formation { display: flex; flex-direction: column; gap: 10px; }
        .fe-form-row { display: flex; align-items: center; gap: 24px; }
        .fe-form-row-left { display: flex; align-items: center; gap: 8px; flex-shrink: 0; width: 100px; }
        .fe-form-row-label { width: 34px; font-size: 0.75rem; color: var(--text-3, #71717a); font-weight: 600; text-transform: uppercase; flex-shrink: 0; }
        .fe-form-slots { flex: 1; display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; }
        .fe-slot-input { width: 50px; text-align: center; }
        .fe-help { font-size: 0.75rem; color: var(--text-3, #71717a); margin: 4px 0; }
        input[type=number]::-webkit-inner-spin-button,
        input[type=number]::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
        input[type=number] { -moz-appearance: textfield; appearance: textfield; }
        .fe-input-player-name { width: 200px; }
        .fe-input-error { border-color: #ef4444 !important; }
        .fe-error-msg { font-size: 0.75rem; color: #ef4444; margin-top: 4px; }
        .fe-btn-home { background: rgba(56,189,248,0.2) !important; border-color: rgba(56,189,248,0.6) !important; color: var(--accent, #38bdf8) !important; }
        .fe-btn-away { background: rgba(74,222,128,0.2) !important; border-color: rgba(74,222,128,0.6) !important; color: #4ade80 !important; }
        .fe-td-actions { display: flex; gap: 4px; align-items: center; }
        .fe-td-actions .fe-action-sep { width: 1px; height: 20px; background: var(--border-1, #3f3f46); margin: 0 4px; flex-shrink: 0; }
        .fe-modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 1000; }
        .fe-modal { background: var(--surface-1, #18181b); border: 1px solid var(--border-1, #3f3f46); border-radius: 10px; padding: 24px; min-width: 320px; max-width: 420px; width: 100%; box-shadow: 0 16px 48px rgba(0,0,0,0.5); }
        .fe-modal-title { font-size: 0.95rem; font-weight: 600; color: var(--text-1, #e4e4e7); margin-bottom: 20px; }
        .fe-modal-field { margin-bottom: 14px; }
        .fe-modal-field label { display: block; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-3, #71717a); margin-bottom: 5px; }
        .fe-modal-colors { display: flex; gap: 16px; }
        .fe-modal-colors .fe-modal-field { flex: 1; }
        .fe-modal-footer { display: flex; justify-content: flex-end; gap: 8px; margin-top: 24px; }
      </style>
      <div class="fe-root">
        <div class="fe-tabs">
          <button class="fe-tab ${this._tab === 'teams' ? 'active' : ''}" data-tab="teams">Teams</button>
          <button class="fe-tab ${this._tab === 'players' ? 'active' : ''}" data-tab="players">Players</button>
          <button class="fe-tab ${this._tab === 'formation' ? 'active' : ''}" data-tab="formation">Starting 11</button>
          <button class="fe-tab ${this._tab === 'media' ? 'active' : ''}" data-tab="media">Media</button>
        </div>
        <div class="fe-content" id="fe-content"></div>

        <!-- Edit-player modal -->
        <div class="fe-modal-backdrop" id="fe-edit-player-modal" style="display:none">
          <div class="fe-modal">
            <div class="fe-modal-title">Edit Player</div>
            <div class="fe-modal-field">
              <label>Shirt Number</label>
              <input class="fe-input fe-input-sm" id="fe-edit-player-num" type="number" min="1" placeholder="#" />
              <div class="fe-error-msg" id="fe-edit-player-num-error" style="display:none"></div>
            </div>
            <div class="fe-modal-field">
              <label>Name</label>
              <input class="fe-input" id="fe-edit-player-name" placeholder="Player name" style="width:100%;box-sizing:border-box" />
            </div>
            <div class="fe-modal-footer">
              <button class="fe-btn" id="fe-edit-player-cancel">Cancel</button>
              <button class="fe-btn fe-btn-primary" id="fe-edit-player-save">Save</button>
            </div>
          </div>
        </div>

        <!-- Formation unsaved-changes confirmation -->
        <div class="fe-modal-backdrop" id="fe-formation-confirm" style="display:none">
          <div class="fe-modal">
            <div class="fe-modal-title">Unsaved Formation</div>
            <p style="font-size:0.85rem;color:var(--text-2,#a1a1aa);margin:0 0 4px">You have unsaved changes to the formation.</p>
            <div class="fe-modal-footer">
              <button class="fe-btn" id="fe-fc-cancel">Cancel</button>
              <button class="fe-btn fe-btn-danger" id="fe-fc-discard">Discard</button>
              <button class="fe-btn fe-btn-primary" id="fe-fc-save">Save & Continue</button>
            </div>
          </div>
        </div>

        <!-- Edit-team modal (lives outside fe-content so re-renders don't destroy it) -->
        <div class="fe-modal-backdrop" id="fe-edit-modal" style="display:none">
          <div class="fe-modal">
            <div class="fe-modal-title">Edit Team</div>
            <div class="fe-modal-field">
              <label>Team Name</label>
              <input class="fe-input" id="fe-edit-name" placeholder="Team name" style="width:100%;box-sizing:border-box" />
            </div>
            <div class="fe-modal-field">
              <label>Short Name</label>
              <input class="fe-input fe-input-sm" id="fe-edit-short" placeholder="Short" />
            </div>
            <div class="fe-modal-field">
              <label>Manager</label>
              <input class="fe-input" id="fe-edit-manager" placeholder="Manager name" style="width:100%;box-sizing:border-box" />
            </div>
            <div class="fe-modal-colors">
              <div class="fe-modal-field">
                <label>Primary Color</label>
                <input class="fe-input fe-input-color" id="fe-edit-color1" type="color" />
              </div>
              <div class="fe-modal-field">
                <label>Secondary Color</label>
                <input class="fe-input fe-input-color" id="fe-edit-color2" type="color" />
              </div>
            </div>
            <div class="fe-modal-footer">
              <button class="fe-btn" id="fe-edit-cancel">Cancel</button>
              <button class="fe-btn fe-btn-primary" id="fe-edit-save">Save</button>
            </div>
          </div>
        </div>
      </div>
    `;

    this.querySelectorAll('.fe-tab').forEach(btn => {
      btn.addEventListener('click', () => {
        const targetTab = btn.dataset.tab;
        if (this._tab === 'formation' && this._formationDirty && targetTab !== 'formation') {
          this._showFormationConfirm(() => {
            this._tab = targetTab;
            this.querySelectorAll('.fe-tab').forEach(b => b.classList.toggle('active', b.dataset.tab === this._tab));
            this._onTabChange();
          });
          return;
        }
        this._tab = targetTab;
        this.querySelectorAll('.fe-tab').forEach(b => b.classList.toggle('active', b.dataset.tab === this._tab));
        this._onTabChange();
      });
    });

    // Formation confirm modal wiring
    const confirmModal = this.querySelector('#fe-formation-confirm');
    this.querySelector('#fe-fc-cancel')?.addEventListener('click', () => {
      confirmModal.style.display = 'none';
      this._formationPendingAction = null;
    });
    this.querySelector('#fe-fc-discard')?.addEventListener('click', async () => {
      await this._loadFormation();
      confirmModal.style.display = 'none';
      const action = this._formationPendingAction;
      this._formationPendingAction = null;
      if (action) action();
    });
    this.querySelector('#fe-fc-save')?.addEventListener('click', async () => {
      await this._saveFormation();
      confirmModal.style.display = 'none';
      const action = this._formationPendingAction;
      this._formationPendingAction = null;
      if (action) action();
    });

    // Player modal wiring
    const playerModal = this.querySelector('#fe-edit-player-modal');
    this.querySelector('#fe-edit-player-cancel')?.addEventListener('click', () => { playerModal.style.display = 'none'; });
    playerModal?.addEventListener('click', (e) => { if (e.target === playerModal) playerModal.style.display = 'none'; });
    this.querySelector('#fe-edit-player-save')?.addEventListener('click', async () => {
      const id = Number(playerModal.dataset.editId);
      if (!id) return;
      const numInput = this.querySelector('#fe-edit-player-num');
      const numError = this.querySelector('#fe-edit-player-num-error');
      const number = Number(numInput?.value);
      const name = this.querySelector('#fe-edit-player-name')?.value?.trim();
      if (!number || !name) return;
      if (this._players.some(p => p.number === number && p.id !== id)) {
        numInput.classList.add('fe-input-error');
        numError.textContent = `#${number} is already taken`;
        numError.style.display = 'block';
        return;
      }
      numInput.classList.remove('fe-input-error');
      numError.style.display = 'none';
      await this._sdk.updateRow('players', id, { number, name });
      playerModal.style.display = 'none';
      await this._loadPlayers();
      this._renderContent();
    });

    // Team modal wiring
    const modal = this.querySelector('#fe-edit-modal');
    this.querySelector('#fe-edit-cancel')?.addEventListener('click', () => { modal.style.display = 'none'; });
    modal?.addEventListener('click', (e) => { if (e.target === modal) modal.style.display = 'none'; });
    this.querySelector('#fe-edit-save')?.addEventListener('click', async () => {
      const id = Number(modal.dataset.editId);
      if (!id) return;
      const name = this.querySelector('#fe-edit-name')?.value?.trim();
      if (!name) return;
      const short_name = this.querySelector('#fe-edit-short')?.value?.trim() || '';
      const manager = this.querySelector('#fe-edit-manager')?.value?.trim() || '';
      const primary_color = this.querySelector('#fe-edit-color1')?.value || '#ffffff';
      const secondary_color = this.querySelector('#fe-edit-color2')?.value || '#000000';
      await this._sdk.updateRow('teams', id, { name, short_name, manager, primary_color, secondary_color });
      // If this team is home or away, sync the updated fields into state too
      const stateUpdates = {};
      if (Number(this._state.home_team_id) === id) {
        stateUpdates.home_team_name = name;
        stateUpdates.home_short_name = short_name;
        stateUpdates.home_manager = manager;
        stateUpdates.home_primary_color = primary_color;
        stateUpdates.home_secondary_color = secondary_color;
      }
      if (Number(this._state.away_team_id) === id) {
        stateUpdates.away_team_name = name;
        stateUpdates.away_short_name = short_name;
        stateUpdates.away_manager = manager;
        stateUpdates.away_primary_color = primary_color;
        stateUpdates.away_secondary_color = secondary_color;
      }
      if (Object.keys(stateUpdates).length) {
        this._state = await this._sdk.setState(stateUpdates);
      }
      modal.style.display = 'none';
      await this._loadTeams();
      this._renderContent();
    });

    this._renderContent();
  }

  async _onTabChange() {
    if (this._tab === 'players') await this._loadPlayers();
    if (this._tab === 'formation') {
      await this._loadPlayers();
      await this._loadFormation();
    }
    this._renderContent();
  }

  _renderContent() {
    const container = this.querySelector('#fe-content');
    if (!container) return;

    if (this._tab === 'teams') this._renderTeams(container);
    else if (this._tab === 'players') this._renderPlayers(container);
    else if (this._tab === 'formation') this._renderFormation(container);
    else if (this._tab === 'media') this._renderMedia(container);
  }

  // ── Teams Tab ──

  _renderTeams(el) {
    el.innerHTML = `
      <div class="fe-section">
        <div class="fe-label">Add Team</div>
        <div class="fe-row">
          <input class="fe-input" id="fe-team-name" placeholder="Team name" />
          <input class="fe-input fe-input-sm" id="fe-team-short" placeholder="Short" />
          <input class="fe-input" id="fe-team-manager" placeholder="Manager name" />
          <input class="fe-input fe-input-color" id="fe-team-color1" type="color" value="#ffffff" title="Primary color" />
          <input class="fe-input fe-input-color" id="fe-team-color2" type="color" value="#000000" title="Secondary color" />
          <button class="fe-btn fe-btn-primary" id="fe-team-add">Add</button>
        </div>
      </div>
      <table class="fe-table">
        <thead><tr><th>Name</th><th>Short</th><th>Colors</th><th></th></tr></thead>
        <tbody>
          ${this._teams.map(t => {
            const isHome = Number(this._state.home_team_id) === t.id;
            const isAway = Number(this._state.away_team_id) === t.id;
            return `
            <tr>
              <td>${this._esc(t.name)}</td>
              <td>${this._esc(t.short_name)}</td>
              <td><span class="fe-color-dot" style="background:${t.primary_color}"></span> <span class="fe-color-dot" style="background:${t.secondary_color}"></span></td>
              <td>
                <div class="fe-td-actions">
                  <button class="fe-btn ${isHome ? 'fe-btn-home' : ''}" data-set-team="${t.id}" data-team-name="${this._esc(t.name)}" data-team-short="${this._esc(t.short_name)}" data-team-manager="${this._esc(t.manager || '')}" data-team-color="${t.primary_color}" data-team-color2="${t.secondary_color}" title="Set as Home">Home</button>
                  <button class="fe-btn ${isAway ? 'fe-btn-away' : ''}" data-set-away="${t.id}" data-team-name="${this._esc(t.name)}" data-team-short="${this._esc(t.short_name)}" data-team-manager="${this._esc(t.manager || '')}" data-team-color="${t.primary_color}" data-team-color2="${t.secondary_color}" title="Set as Away">Away</button>
                  <div class="fe-action-sep"></div>
                  <button class="fe-btn" data-edit-team="${t.id}">Edit</button>
                  <button class="fe-btn fe-btn-danger" data-del-team="${t.id}">Del</button>
                </div>
              </td>
            </tr>`;
          }).join('')}
        </tbody>
      </table>
    `;

    el.querySelector('#fe-team-add')?.addEventListener('click', async () => {
      const name = el.querySelector('#fe-team-name')?.value?.trim();
      if (!name) return;
      const short_name = el.querySelector('#fe-team-short')?.value?.trim() || '';
      const manager = el.querySelector('#fe-team-manager')?.value?.trim() || '';
      const primary_color = el.querySelector('#fe-team-color1')?.value || '#ffffff';
      const secondary_color = el.querySelector('#fe-team-color2')?.value || '#000000';
      await this._sdk.insertRow('teams', { name, short_name, manager, primary_color, secondary_color });
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
        this._state = await this._sdk.setState({
          home_team_id: Number(btn.dataset.setTeam),
          home_team_name: btn.dataset.teamName,
          home_short_name: btn.dataset.teamShort,
          home_manager: btn.dataset.teamManager || '',
          home_primary_color: btn.dataset.teamColor,
          home_secondary_color: btn.dataset.teamColor2,
        });
        this._renderContent();
      });
    });

    el.querySelectorAll('[data-set-away]').forEach(btn => {
      btn.addEventListener('click', async () => {
        this._state = await this._sdk.setState({
          away_team_id: Number(btn.dataset.setAway),
          away_team_name: btn.dataset.teamName,
          away_short_name: btn.dataset.teamShort,
          away_manager: btn.dataset.teamManager || '',
          away_primary_color: btn.dataset.teamColor,
          away_secondary_color: btn.dataset.teamColor2,
        });
        this._renderContent();
      });
    });

    el.querySelectorAll('[data-edit-team]').forEach(btn => {
      btn.addEventListener('click', () => {
        const team = this._teams.find(t => t.id === Number(btn.dataset.editTeam));
        if (!team) return;
        const modal = this.querySelector('#fe-edit-modal');
        modal.dataset.editId = team.id;
        this.querySelector('#fe-edit-name').value = team.name;
        this.querySelector('#fe-edit-short').value = team.short_name;
        this.querySelector('#fe-edit-manager').value = team.manager || '';
        this.querySelector('#fe-edit-color1').value = team.primary_color;
        this.querySelector('#fe-edit-color2').value = team.secondary_color;
        modal.style.display = 'flex';
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
            <div style="display:flex;flex-direction:column;gap:2px">
              <input class="fe-input fe-input-sm" id="fe-player-num" type="number" min="1" placeholder="#" />
              <div class="fe-error-msg" id="fe-player-num-error" style="display:none"></div>
            </div>
            <input class="fe-input fe-input-player-name" id="fe-player-name" placeholder="Name" />
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
                <td>
                  <div class="fe-td-actions">
                    <button class="fe-btn" data-edit-player="${p.id}">Edit</button>
                    <button class="fe-btn fe-btn-danger" data-del-player="${p.id}">Del</button>
                  </div>
                </td>
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
      const numInput = el.querySelector('#fe-player-num');
      const numError = el.querySelector('#fe-player-num-error');
      const number = Number(numInput?.value);
      const name = el.querySelector('#fe-player-name')?.value?.trim();
      if (!number || !name) return;
      if (this._players.some(p => p.number === number)) {
        numInput.classList.add('fe-input-error');
        numError.textContent = `#${number} is already taken`;
        numError.style.display = 'block';
        return;
      }
      numInput.classList.remove('fe-input-error');
      numError.style.display = 'none';
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

    el.querySelectorAll('[data-edit-player]').forEach(btn => {
      btn.addEventListener('click', () => {
        const player = this._players.find(p => p.id === Number(btn.dataset.editPlayer));
        if (!player) return;
        const modal = this.querySelector('#fe-edit-player-modal');
        modal.dataset.editId = player.id;
        this.querySelector('#fe-edit-player-num').value = player.number;
        this.querySelector('#fe-edit-player-name').value = player.name;
        modal.style.display = 'flex';
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
              <div class="fe-form-row-left">
                <span class="fe-form-row-label">${label}</span>
                ${ri === 0 ? '' : `<select class="fe-select fe-row-count" data-row="${ri}" style="width:50px">
                  ${[0,1,2,3,4,5,6].map(n => `<option value="${n}" ${n === rows[ri] ? 'selected' : ''}>${n}</option>`).join('')}
                </select>`}
              </div>
              <div class="fe-form-slots" data-slots="${ri}">
                ${Array.from({length: rows[ri]}, (_, si) => `
                  <input class="fe-input fe-slot-input" type="number" min="1"
                    data-row="${ri}" data-slot="${si}"
                    value="${slots[ri][si] || ''}"
                    placeholder="#" />
                `).join('')}
              </div>
            </div>`
          ).join('')}
        </div>
        <div class="fe-row" style="margin-top:12px;">
          <button class="fe-btn fe-btn-primary" id="fe-form-save">Save Formation</button>
        </div>
      `}
    `;

    el.querySelector('#fe-form-team')?.addEventListener('change', async (e) => {
      const newTeamId = Number(e.target.value);
      if (this._formationDirty) {
        // Revert the dropdown visually until the user decides
        e.target.value = this._selectedTeamId;
        this._showFormationConfirm(async () => {
          this._selectedTeamId = newTeamId;
          await this._loadPlayers();
          await this._loadFormation();
          this._renderContent();
        });
        return;
      }
      this._selectedTeamId = newTeamId;
      await this._loadPlayers();
      await this._loadFormation();
      this._renderContent();
    });

    // Validate slot inputs and mark dirty on change
    const wireSlotInputs = (inputs) => {
      inputs.forEach(input => {
        this._validateSlotInput(input);
        input.addEventListener('input', () => {
          this._formationDirty = true;
          this._validateSlotInput(input);
        });
      });
    };
    wireSlotInputs(el.querySelectorAll('.fe-slot-input'));

    // Row count changes: mark dirty, re-render slots, wire new inputs
    el.querySelectorAll('.fe-row-count').forEach(sel => {
      sel.addEventListener('change', () => {
        this._formationDirty = true;
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
        wireSlotInputs(container.querySelectorAll('.fe-slot-input'));
      });
    });

    el.querySelector('#fe-form-save')?.addEventListener('click', async () => {
      await this._saveFormation();
      this._renderContent();
    });
  }

  async _saveFormation() {
    const el = this.querySelector('#fe-content');
    if (!el || !this._selectedTeamId) return;
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
    for (const f of this._formation) await this._sdk.deleteRow('formation', f.id);
    for (const entry of entries) await this._sdk.insertRow('formation', entry);
    await this._loadFormation(); // also resets _formationDirty
  }

  _showFormationConfirm(callback) {
    this._formationPendingAction = callback;
    this.querySelector('#fe-formation-confirm').style.display = 'flex';
  }

  _validateSlotInput(input) {
    const val = input.value.trim();
    if (!val) { input.classList.remove('fe-input-error'); return; }
    const valid = this._players.some(p => p.number === Number(val));
    input.classList.toggle('fe-input-error', !valid);
  }

  // ── Media Tab ──

  _renderMedia(el) {
    const hasLogo = !!this._state.logo_data_url;
    el.innerHTML = `
      <div class="fe-section">
        <div class="fe-label">Scoreboard Logo</div>
        <div class="fe-row" style="align-items:flex-start;gap:20px;flex-wrap:wrap">

          <div id="fe-logo-preview-wrap" style="flex-shrink:0">
            ${hasLogo
              ? `<div style="display:flex;flex-direction:column;align-items:center;gap:8px">
                   <img id="fe-logo-preview" style="width:100px;height:100px;object-fit:contain;border:1px solid var(--border-1,#3f3f46);border-radius:8px;background:#111;padding:8px" />
                   <button class="fe-btn fe-btn-danger" id="fe-logo-remove">Remove</button>
                 </div>`
              : `<div style="width:100px;height:100px;display:flex;align-items:center;justify-content:center;border:1px dashed var(--border-1,#3f3f46);border-radius:8px;color:var(--text-3,#71717a);font-size:0.75rem;text-align:center;padding:8px;line-height:1.4">
                   No logo set
                 </div>`
            }
          </div>

          <div style="display:flex;flex-direction:column;gap:8px;padding-top:2px">
            <label class="fe-btn fe-btn-primary" style="cursor:pointer;display:inline-flex;align-items:center;gap:6px">
              <input type="file" accept="image/*" id="fe-logo-upload" style="display:none" />
              ${hasLogo ? 'Replace image' : 'Upload image'}
            </label>
            <p class="fe-help">PNG, JPG, WEBP or SVG. Scaled to fit the logo cell.</p>
          </div>

        </div>
      </div>
    `;

    // Set preview src via JS — avoids embedding the full data URL in innerHTML
    const previewImg = el.querySelector('#fe-logo-preview');
    if (previewImg && this._state.logo_data_url) previewImg.src = this._state.logo_data_url;

    el.querySelector('#fe-logo-upload')?.addEventListener('change', async (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const dataUrl = await this._resizeImage(file, 240);
      this._state = await this._sdk.setState({ logo_data_url: dataUrl });
      this._renderContent();
    });

    el.querySelector('#fe-logo-remove')?.addEventListener('click', async () => {
      this._state = await this._sdk.setState({ logo_data_url: null });
      this._renderContent();
    });
  }

  _resizeImage(file, maxSize) {
    return new Promise((resolve) => {
      const img = new Image();
      const url = URL.createObjectURL(file);
      img.onload = () => {
        const scale = Math.min(maxSize / img.width, maxSize / img.height, 1);
        const canvas = document.createElement('canvas');
        canvas.width  = Math.round(img.width  * scale);
        canvas.height = Math.round(img.height * scale);
        canvas.getContext('2d').drawImage(img, 0, 0, canvas.width, canvas.height);
        URL.revokeObjectURL(url);
        resolve(canvas.toDataURL('image/png'));
      };
      img.src = url;
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
