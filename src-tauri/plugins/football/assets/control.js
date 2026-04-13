/**
 * Football Control Panel – Custom Element
 *
 * Touch-first layout: all interactive targets are ≥ 52 px tall, buttons fill
 * available width via the shared `ctrl-*` system injected by PluginHost.
 *
 * This file only defines football-specific structure and colours; the button
 * sizing, grid helpers, and input styles come from the shared stylesheet.
 */
class FootballControl extends HTMLElement {
  constructor() {
    super();
    this._sdk = null;
    this._state = {};
    this._unsub = null;
    this._timerRAF = null;
    this._selectedTeam = 'home';
    this._pendingEventType = null;
  }

  set sdk(val) {
    this._sdk = val;
    this._init();
  }

  async _init() {
    this._state = await this._sdk.getState();
    this._render();
    this._unsub = this._sdk.onStateChanged((state) => {
      this._state = state;
      this._updateDisplay();
    });
    this._startTimerLoop();
  }

  disconnectedCallback() {
    if (this._unsub) this._unsub();
    if (this._timerRAF) cancelAnimationFrame(this._timerRAF);
  }

  _getTimerSeconds() {
    const s = this._state;
    let t = Number(s.timer_accumulated) || 0;
    if (s.timer_running && s.timer_start_epoch) {
      t += Math.floor(Date.now() / 1000) - Number(s.timer_start_epoch);
    }
    return Math.max(0, t);
  }

  _formatTime(totalSec) {
    const m = Math.floor(totalSec / 60);
    const s = totalSec % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }

  _startTimerLoop() {
    const tick = () => {
      const el = this.querySelector('.fc-timer-clock');
      if (el) el.textContent = this._formatTime(this._getTimerSeconds());
      this._timerRAF = requestAnimationFrame(tick);
    };
    this._timerRAF = requestAnimationFrame(tick);
  }

  _render() {
    const s = this._state;
    const isRunning = !!s.timer_running;

    this.innerHTML = `
      <style>
        /*
         * Football-specific layout only.
         * All ctrl-btn / ctrl-row / ctrl-grid-* / ctrl-input styles come from
         * the shared stylesheet injected by PluginHost (control-styles.ts).
         */
        .fc-root {
          display: flex;
          flex-direction: column;
          gap: 10px;
          font-family: inherit;
          color: var(--text-1, #e4e4e7);
        }

        /* ── Section card ─────────────────────────────────────────── */
        .fc-section {
          background: var(--surface-2, #27272a);
          border: 1px solid var(--border-1, #3f3f46);
          border-radius: 12px;
          padding: 14px;
          display: flex;
          flex-direction: column;
          gap: 10px;
        }
        .fc-section-title {
          font-size: 0.62rem;
          font-weight: 700;
          text-transform: uppercase;
          letter-spacing: 0.1em;
          color: var(--text-3, #71717a);
        }

        /* ── Score ────────────────────────────────────────────────── */
        .fc-scores {
          display: flex;
          align-items: stretch;
          gap: 8px;
        }
        .fc-team {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 6px;
          flex: 1;
          min-width: 0;
        }
        .fc-team-name {
          font-size: 0.8rem;
          font-weight: 700;
          color: var(--text-2, #a1a1aa);
          text-align: center;
          text-transform: uppercase;
          letter-spacing: 0.05em;
          width: 100%;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .fc-score {
          font-size: 3.5rem;
          font-weight: 900;
          font-variant-numeric: tabular-nums;
          line-height: 1;
        }
        .fc-separator {
          font-size: 2.25rem;
          font-weight: 300;
          color: var(--text-3, #71717a);
          align-self: center;
          /* nudge down so it sits next to the score digits, not the name */
          margin-top: 28px;
        }

        /* ── Timer ────────────────────────────────────────────────── */
        .fc-timer-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 8px;
          align-items: center;
        }
        .fc-timer-clock {
          font-size: 2.5rem;
          font-weight: 800;
          font-variant-numeric: tabular-nums;
          letter-spacing: -0.02em;
          line-height: 1;
          text-align: center;
        }
        .fc-set-inputs {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 4px;
        }
        .fc-colon {
          font-size: 1.3rem;
          font-weight: 700;
          color: var(--text-3, #71717a);
          flex-shrink: 0;
        }

        /* Hide browser default spinners on number inputs */
        input[type="number"]::-webkit-outer-spin-button,
        input[type="number"]::-webkit-inner-spin-button {
          -webkit-appearance: none;
          margin: 0;
        }
        input[type="number"] {
          -moz-appearance: textfield;
        }

        /* ── Events ───────────────────────────────────────────────── */
        .fc-event-form {
          display: none;
          flex-direction: column;
          gap: 8px;
        }
        .fc-event-form.active { display: flex; }

        /* Event type button selection feedback */
        .fc-event-btns .ctrl-btn {
          transition: box-shadow 0.15s;
        }
        /* White ring around the chosen type */
        .fc-event-active {
          box-shadow: 0 0 0 2px #fff, 0 0 8px 2px rgba(255, 255, 255, 0.25) !important;
        }

        /* Confirm rows inside the player picker – hidden until needed */
        .fc-single-player-row,
        .fc-confirm-goal,
        .fc-confirm-card,
        .fc-sub-row { display: none; }

        /* Substitution button colour – mirrors the ctrl-btn-* pattern */
        .fc-btn-sub       { background: rgba(168,85,247,0.14); border-color: rgba(168,85,247,0.35); color: #a855f7; }
        .fc-btn-sub:hover { background: rgba(168,85,247,0.26); }

        /* Coloured borders for substitution inputs */
        .fc-input-in {
          border-color: #22c55e !important;
          caret-color:  #22c55e;
        }
        .fc-input-in:focus { outline-color: #22c55e !important; }
        .fc-input-out {
          border-color: #ef4444 !important;
          caret-color:  #ef4444;
        }
        .fc-input-out:focus { outline-color: #ef4444 !important; }

        /* truncate long team names inside team-selector buttons */
        .fc-team-btn {
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        /* ── Team selector (dynamic events) ──────────────────────── */
        /* Slightly shorter than a full ctrl-btn-lg */
        .fc-team-sel {
          min-height: 38px !important;
          padding-top: 5px !important;
          padding-bottom: 5px !important;
        }
        /* Blue active state instead of the default green */
        .fc-team-sel-active {
          background: var(--accent, #3b82f6) !important;
          border-color: var(--accent, #3b82f6) !important;
          color: #fff !important;
          box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #3b82f6) 40%, transparent) !important;
        }

        /* ── Lineup buttons (static events) ──────────────────────── */
        .fc-lineup-btn {
          display: flex !important;
          flex-direction: column;
          align-items: center;
          gap: 1px;
          height: auto !important;
        }
        .fc-lineup-team {
          font-weight: 700;
          font-size: 0.9em;
          max-width: 100%;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .fc-lineup-label {
          font-size: 0.65em;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.08em;
          opacity: 0.7;
        }
      </style>

      <div class="fc-root">

        <!-- ══ Score ══════════════════════════════════════════════ -->
        <div class="fc-section">
          <div class="fc-section-title">Score</div>
          <div class="fc-scores">

            <div class="fc-team">
              <span class="fc-team-name home-name">${this._esc(s.home_team_name || 'Home')}</span>
              <span class="fc-score home-score">${Number(s.home_score) || 0}</span>
              <div class="ctrl-row">
                <button class="ctrl-btn ctrl-btn-sq" data-action="score" data-team="home" data-delta="-1">−</button>
                <button class="ctrl-btn ctrl-btn-sq" data-action="score" data-team="home" data-delta="1">+</button>
              </div>
            </div>
            <div class="fc-team">
              <span class="fc-team-name away-name">${this._esc(s.away_team_name || 'Away')}</span>
              <span class="fc-score away-score">${Number(s.away_score) || 0}</span>
              <div class="ctrl-row">
                <button class="ctrl-btn ctrl-btn-sq" data-action="score" data-team="away" data-delta="-1">−</button>
                <button class="ctrl-btn ctrl-btn-sq" data-action="score" data-team="away" data-delta="1">+</button>
              </div>
            </div>

          </div>
        </div>

        <!-- ══ Timer ══════════════════════════════════════════════ -->
        <div class="fc-section">
          <div class="fc-section-title">Timer</div>

          <div class="fc-timer-grid">

            <!-- Row 1: clock | play/stop -->
            <span class="fc-timer-clock">${this._formatTime(this._getTimerSeconds())}</span>
            <button
              class="ctrl-btn ctrl-btn-lg ctrl-btn-full${isRunning ? ' ctrl-btn-danger' : ''}"
              data-action="timer-toggle"
            >${isRunning ? '⏸  Stop' : '▶  Play'}</button>

            <!-- Row 2: mm/ss inputs | set -->
            <div class="fc-set-inputs">
              <input class="ctrl-input ctrl-input-num" id="fc-set-min" type="number" min="0" max="999" value="0" placeholder="mm" />
              <span class="fc-colon">:</span>
              <input class="ctrl-input ctrl-input-num" id="fc-set-sec" type="number" min="0" max="59"  value="0" placeholder="ss" />
            </div>
            <button class="ctrl-btn ctrl-btn-lg ctrl-btn-full" data-action="timer-set">Set</button>

          </div>
        </div>

        <!-- ══ Dynamic Events ════════════════════════════════════ -->
        <div class="fc-section">
          <div class="fc-section-title">Dynamic Events</div>

          <!-- Team selector – active button shows which team is targeted -->
          <div class="ctrl-row">
            <button class="ctrl-btn ctrl-btn-lg fc-team-btn fc-team-sel${this._selectedTeam === 'home' ? ' fc-team-sel-active' : ''}"
                    data-action="select-team" data-team="home">${this._esc(s.home_team_name || 'Home')}</button>
            <button class="ctrl-btn ctrl-btn-lg fc-team-btn fc-team-sel${this._selectedTeam === 'away' ? ' fc-team-sel-active' : ''}"
                    data-action="select-team" data-team="away">${this._esc(s.away_team_name || 'Away')}</button>
          </div>

          <!-- Event type buttons -->
          <div class="ctrl-row fc-event-btns">
            <button class="ctrl-btn ctrl-btn-lg ctrl-btn-goal"   data-action="event-goal">Goal</button>
            <button class="ctrl-btn ctrl-btn-lg ctrl-btn-warn"   data-action="event-card">Card</button>
            <button class="ctrl-btn ctrl-btn-lg fc-btn-sub" data-action="event-sub">Substitution</button>
          </div>

          <!-- Player picker – shown when an event button is tapped -->
          <div class="fc-event-form" id="fc-event-form">

            <!-- Goal / Card: single player number + context-specific confirm(s) -->
            <div class="ctrl-row fc-single-player-row">
              <input class="ctrl-input ctrl-input-num" id="fc-event-number" type="number" min="1" placeholder="Player #" />
              <button class="ctrl-btn ctrl-btn-lg fc-confirm-goal"               data-action="confirm-goal">Confirm</button>
              <button class="ctrl-btn ctrl-btn-lg ctrl-btn-warn  fc-confirm-card" data-action="confirm-card" data-card-type="yellow">Yellow Card</button>
              <button class="ctrl-btn ctrl-btn-lg ctrl-btn-danger fc-confirm-card" data-action="confirm-card" data-card-type="red">Red Card</button>
            </div>

            <!-- Substitution: player-in (green) + player-out (red) + confirm -->
            <div class="ctrl-row fc-sub-row">
              <input class="ctrl-input ctrl-input-num fc-input-in"  id="fc-sub-in"  type="number" min="1" placeholder="In #" />
              <input class="ctrl-input ctrl-input-num fc-input-out" id="fc-sub-out" type="number" min="1" placeholder="Out #" />
              <button class="ctrl-btn ctrl-btn-lg" data-action="confirm-sub">Confirm</button>
            </div>

          </div>
        </div>

        <!-- ══ Static Events ══════════════════════════════════════ -->
        <div class="fc-section">
          <div class="fc-section-title">Static Events</div>

          <div class="ctrl-row">
            <button class="ctrl-btn ctrl-btn-lg ctrl-btn-accent fc-lineup-btn"
                    data-action="show-lineup" data-team="home">
              <span class="fc-lineup-team">${this._esc(s.home_team_name || 'Home')}</span>
              <span class="fc-lineup-label">Lineup</span>
            </button>
            <button class="ctrl-btn ctrl-btn-lg ctrl-btn-accent fc-lineup-btn"
                    data-action="show-lineup" data-team="away">
              <span class="fc-lineup-team">${this._esc(s.away_team_name || 'Away')}</span>
              <span class="fc-lineup-label">Lineup</span>
            </button>
          </div>
        </div>

      </div>
    `;
    this._bindEvents();
  }

  _bindEvents() {
    this.addEventListener('click', async (e) => {
      const btn = e.target.closest('[data-action]');
      if (!btn) return;
      const action = btn.dataset.action;

      if (action === 'score') {
        const team = btn.dataset.team;
        const delta = Number(btn.dataset.delta);
        const key = team === 'home' ? 'home_score' : 'away_score';
        const current = Number(this._state[key]) || 0;
        const newScore = Math.max(0, current + delta);
        // Optimistic local update: rapid clicks read the already-updated value
        // instead of a stale one, preventing all requests from carrying the same
        // number and later in-flight requests from overwriting newer ones.
        this._state = { ...this._state, [key]: newScore };
        this._updateDisplay();
        await this._sdk.setState({ [key]: newScore });
      }

      if (action === 'timer-toggle') {
        if (this._state.timer_running) {
          const elapsed = this._getTimerSeconds();
          // Optimistic update prevents a second tap from re-starting the timer
          // before the broadcast arrives.
          this._state = { ...this._state, timer_running: false, timer_accumulated: elapsed, timer_start_epoch: null };
          this._updateDisplay();
          await this._sdk.setState({
            timer_running: false,
            timer_accumulated: elapsed,
            timer_start_epoch: null,
          });
        } else {
          const epoch = Math.floor(Date.now() / 1000);
          this._state = { ...this._state, timer_running: true, timer_start_epoch: epoch };
          this._updateDisplay();
          await this._sdk.setState({
            timer_running: true,
            timer_start_epoch: epoch,
          });
        }
      }

      if (action === 'timer-reset') {
        this._state = { ...this._state, timer_running: false, timer_accumulated: 0, timer_start_epoch: null };
        this._updateDisplay();
        await this._sdk.setState({
          timer_running: false,
          timer_accumulated: 0,
          timer_start_epoch: null,
        });
      }

      if (action === 'timer-set') {
        const minEl = this.querySelector('#fc-set-min');
        const secEl = this.querySelector('#fc-set-sec');
        const m = Number(minEl?.value) || 0;
        const s = Number(secEl?.value) || 0;
        const total = m * 60 + s;
        const updates = { timer_accumulated: total };
        if (this._state.timer_running) {
          updates.timer_start_epoch = Math.floor(Date.now() / 1000);
        }
        this._state = { ...this._state, ...updates };
        await this._sdk.setState(updates);
      }

      if (action === 'select-team') {
        this._selectedTeam = btn.dataset.team;
        this.querySelectorAll('[data-action="select-team"]').forEach(b => {
          b.classList.toggle('fc-team-sel-active', b.dataset.team === this._selectedTeam);
        });
      }

      if (action === 'event-goal' || action === 'event-card' || action === 'event-sub') {
        this._pendingEventType = action.replace('event-', '');
        this.querySelector('#fc-event-form')?.classList.add('active');
        this._updateEventSelection();
      }

      if (action === 'show-lineup') {
        await this._sdk.triggerPopup('lineup', { team: btn.dataset.team });
      }

      if (action === 'confirm-goal' || action === 'confirm-card') {
        const number = Number(this.querySelector('#fc-event-number')?.value) || 0;
        if (!number) return;

        const team   = this._selectedTeam;
        const teamId = team === 'home' ? this._state.home_team_id : this._state.away_team_id;
        let playerName = `#${number}`;
        if (teamId) {
          try {
            const players = await this._sdk.queryData('players', {
              team_id: String(teamId),
              number:  String(number),
            });
            if (players.length > 0) playerName = players[0].name;
          } catch { /* fallback */ }
        }

        const teamName  = team === 'home' ? this._state.home_team_name  : this._state.away_team_name;
        const teamColor = team === 'home' ? this._state.home_primary_color : this._state.away_primary_color;

        if (action === 'confirm-goal') {
          const key      = team === 'home' ? 'home_score' : 'away_score';
          const newScore = (Number(this._state[key]) || 0) + 1;
          this._state    = { ...this._state, [key]: newScore };
          await this._sdk.setState({ [key]: newScore });
          await this._sdk.triggerPopup('goal-popup', {
            player_name:   playerName,
            player_number: String(number),
            team_name:     teamName  || team.toUpperCase(),
            team_color:    teamColor || '#ffffff',
          });
          this._sdk.fireEvent('goal', { team, player_number: number, player_name: playerName });
        } else {
          const cardType = btn.dataset.cardType; // 'yellow' | 'red'
          await this._sdk.triggerPopup('card-popup', {
            card_type:     cardType === 'yellow' ? 'Yellow Card' : 'Red Card',
            card_color:    cardType === 'yellow' ? '#eab308'     : '#ef4444',
            player_name:   playerName,
            player_number: String(number),
            team_name:     teamName  || team.toUpperCase(),
            team_color:    teamColor || '#ffffff',
          });
          this._sdk.fireEvent(cardType + '-card', { team, player_number: number, player_name: playerName });
        }

        this._pendingEventType = null;
        this._updateEventSelection();
        this.querySelector('#fc-event-form')?.classList.remove('active');
        const numInput = this.querySelector('#fc-event-number');
        if (numInput) numInput.value = '';
      }

      if (action === 'confirm-sub') {
        const numIn  = Number(this.querySelector('#fc-sub-in')?.value)  || 0;
        const numOut = Number(this.querySelector('#fc-sub-out')?.value) || 0;
        if (!numIn || !numOut) return;

        const team   = this._selectedTeam;
        const teamId = team === 'home' ? this._state.home_team_id : this._state.away_team_id;

        const resolveName = async (number) => {
          if (!teamId) return `#${number}`;
          try {
            const rows = await this._sdk.queryData('players', { team_id: String(teamId), number: String(number) });
            return rows.length > 0 ? rows[0].name : `#${number}`;
          } catch { return `#${number}`; }
        };

        const [nameIn, nameOut] = await Promise.all([resolveName(numIn), resolveName(numOut)]);
        const teamName  = team === 'home' ? this._state.home_team_name     : this._state.away_team_name;
        const teamColor = team === 'home' ? this._state.home_primary_color : this._state.away_primary_color;

        await this._sdk.triggerPopup('sub-popup', {
          player_in_number:  String(numIn),
          player_in_name:    nameIn,
          player_out_number: String(numOut),
          player_out_name:   nameOut,
          team_name:         teamName  || team.toUpperCase(),
          team_color:        teamColor || '#ffffff',
        });
        this._sdk.fireEvent('substitution', {
          team,
          player_in:  { number: numIn,  name: nameIn  },
          player_out: { number: numOut, name: nameOut },
        });

        this._pendingEventType = null;
        this._updateEventSelection();
        this.querySelector('#fc-event-form')?.classList.remove('active');
        const inInput  = this.querySelector('#fc-sub-in');
        const outInput = this.querySelector('#fc-sub-out');
        if (inInput)  inInput.value  = '';
        if (outInput) outInput.value = '';
      }
    });

  }

  _updateEventSelection() {
    const row = this.querySelector('.fc-event-btns');
    if (!row) return;

    row.classList.toggle('has-selection', this._pendingEventType !== null);
    row.querySelectorAll('[data-action^="event-"]').forEach(b => {
      const type = b.dataset.action.replace('event-', '');
      b.classList.toggle('fc-event-active', type === this._pendingEventType);
    });

    // Show the right row and confirm button(s) inside the player picker
    const isSinglePlayer = this._pendingEventType === 'goal' || this._pendingEventType === 'card';
    const singleRow = this.querySelector('.fc-single-player-row');
    const subRow    = this.querySelector('.fc-sub-row');
    if (singleRow) singleRow.style.display = isSinglePlayer                          ? 'flex' : 'none';
    if (subRow)    subRow.style.display    = this._pendingEventType === 'sub'         ? 'flex' : 'none';

    const goalBtn  = this.querySelector('.fc-confirm-goal');
    const cardBtns = this.querySelectorAll('.fc-confirm-card');
    if (goalBtn) goalBtn.style.display = this._pendingEventType === 'goal' ? 'flex' : 'none';
    cardBtns.forEach(b => { b.style.display = this._pendingEventType === 'card' ? 'flex' : 'none'; });
  }

  _updateDisplay() {
    const s         = this._state;
    const isRunning = !!s.timer_running;

    const setText = (sel, text) => {
      const el = this.querySelector(sel);
      if (el) el.textContent = text;
    };

    setText('.home-name',    s.home_team_name || 'Home');
    setText('.away-name',    s.away_team_name || 'Away');
    setText('.home-score',   String(Number(s.home_score) || 0));
    setText('.away-score',   String(Number(s.away_score) || 0));

    const playBtn = this.querySelector('[data-action="timer-toggle"]');
    if (playBtn) {
      playBtn.textContent = isRunning ? '⏸  Stop' : '▶  Play';
      playBtn.classList.toggle('ctrl-btn-danger', isRunning);
    }

    // Dynamic events – team selector buttons
    this.querySelectorAll('[data-action="select-team"]').forEach(b => {
      b.textContent = b.dataset.team === 'home' ? (s.home_team_name || 'Home') : (s.away_team_name || 'Away');
      b.classList.toggle('fc-team-sel-active', b.dataset.team === this._selectedTeam);
    });

    // Static events – lineup buttons (update only the team-name span)
    this.querySelectorAll('[data-action="show-lineup"]').forEach(b => {
      const nameEl = b.querySelector('.fc-lineup-team');
      if (nameEl) nameEl.textContent = b.dataset.team === 'home' ? (s.home_team_name || 'Home') : (s.away_team_name || 'Away');
    });
  }

  _esc(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('football-control', FootballControl);
export default FootballControl;
