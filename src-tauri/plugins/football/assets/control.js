/**
 * Football Control Panel – Custom Element
 * Provides score controls, timer, and event triggers (goal, cards, lineup).
 */
class FootballControl extends HTMLElement {
  constructor() {
    super();
    this._sdk = null;
    this._state = {};
    this._unsub = null;
    this._timerRAF = null;
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
      const el = this.querySelector('.timer-display');
      if (el) el.textContent = this._formatTime(this._getTimerSeconds());
      this._timerRAF = requestAnimationFrame(tick);
    };
    this._timerRAF = requestAnimationFrame(tick);
  }

  _render() {
    const s = this._state;
    this.innerHTML = `
      <style>
        :host, .fc-root { font-family: inherit; color: var(--text-1, #e4e4e7); }
        .fc-root { display: flex; flex-direction: column; gap: 16px; }
        .fc-section { background: var(--surface-2, #27272a); border: 1px solid var(--border-1, #3f3f46); border-radius: 8px; padding: 12px; }
        .fc-section-title { font-size: 0.7rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-3, #71717a); margin-bottom: 8px; }
        .fc-scores { display: flex; align-items: center; justify-content: center; gap: 20px; }
        .fc-team { display: flex; flex-direction: column; align-items: center; gap: 4px; flex: 1; }
        .fc-team-name { font-size: 0.8rem; font-weight: 600; color: var(--text-2, #a1a1aa); text-align: center; }
        .fc-score { font-size: 2rem; font-weight: 800; font-variant-numeric: tabular-nums; }
        .fc-score-btns { display: flex; gap: 4px; }
        .fc-btn { padding: 4px 10px; background: var(--surface-3, #3f3f46); border: 1px solid var(--border-2, #52525b); border-radius: 6px; color: var(--text-1, #e4e4e7); cursor: pointer; font-size: 0.85rem; font-weight: 600; font-family: inherit; transition: background 0.1s; }
        .fc-btn:hover { background: var(--border-1, #3f3f46); }
        .fc-btn:active { background: var(--border-2, #52525b); }
        .fc-separator { font-size: 1.5rem; font-weight: 300; color: var(--text-3, #71717a); }
        .fc-timer { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
        .timer-display { font-size: 1.5rem; font-weight: 700; font-variant-numeric: tabular-nums; min-width: 80px; text-align: center; }
        .fc-half { font-size: 0.75rem; color: var(--text-3, #71717a); background: var(--surface-3, #3f3f46); padding: 2px 8px; border-radius: 4px; }
        .fc-timer-controls { display: flex; gap: 6px; flex-wrap: wrap; }
        .fc-events { display: flex; gap: 6px; flex-wrap: wrap; }
        .fc-btn-goal { background: var(--go-dim, rgba(34,197,94,0.15)); border-color: rgba(34,197,94,0.3); color: #22c55e; }
        .fc-btn-goal:hover { background: rgba(34,197,94,0.25); }
        .fc-btn-yellow { background: rgba(234,179,8,0.15); border-color: rgba(234,179,8,0.3); color: #eab308; }
        .fc-btn-yellow:hover { background: rgba(234,179,8,0.25); }
        .fc-btn-red { background: var(--live-dim, rgba(239,68,68,0.15)); border-color: rgba(239,68,68,0.3); color: #ef4444; }
        .fc-btn-red:hover { background: rgba(239,68,68,0.25); }
        .fc-btn-lineup { background: var(--accent-dim, rgba(56,189,248,0.15)); border-color: rgba(56,189,248,0.3); color: #38bdf8; }
        .fc-btn-lineup:hover { background: rgba(56,189,248,0.25); }
        .fc-input-row { display: flex; align-items: center; gap: 6px; margin-top: 6px; }
        .fc-select, .fc-input { padding: 4px 8px; background: var(--surface-3, #3f3f46); border: 1px solid var(--border-2, #52525b); border-radius: 6px; color: var(--text-1, #e4e4e7); font-size: 0.8rem; font-family: inherit; }
        .fc-input { width: 60px; }
        .fc-set-time { display: flex; align-items: center; gap: 4px; margin-top: 6px; }
        .fc-set-input { width: 50px; padding: 3px 6px; background: var(--surface-3, #3f3f46); border: 1px solid var(--border-2, #52525b); border-radius: 4px; color: var(--text-1, #e4e4e7); font-size: 0.8rem; text-align: center; font-family: inherit; }
      </style>
      <div class="fc-root">
        <!-- Score -->
        <div class="fc-section">
          <div class="fc-section-title">Score</div>
          <div class="fc-scores">
            <div class="fc-team">
              <span class="fc-team-name home-name">${this._esc(s.home_team_name || 'HOME')}</span>
              <span class="fc-score home-score">${Number(s.home_score) || 0}</span>
              <div class="fc-score-btns">
                <button class="fc-btn" data-action="score" data-team="home" data-delta="-1">-</button>
                <button class="fc-btn" data-action="score" data-team="home" data-delta="1">+</button>
              </div>
            </div>
            <span class="fc-separator">-</span>
            <div class="fc-team">
              <span class="fc-team-name away-name">${this._esc(s.away_team_name || 'AWAY')}</span>
              <span class="fc-score away-score">${Number(s.away_score) || 0}</span>
              <div class="fc-score-btns">
                <button class="fc-btn" data-action="score" data-team="away" data-delta="-1">-</button>
                <button class="fc-btn" data-action="score" data-team="away" data-delta="1">+</button>
              </div>
            </div>
          </div>
        </div>

        <!-- Timer -->
        <div class="fc-section">
          <div class="fc-section-title">Timer</div>
          <div class="fc-timer">
            <span class="timer-display">${this._formatTime(this._getTimerSeconds())}</span>
            <span class="fc-half">${Number(s.half) === 2 ? '2nd Half' : '1st Half'}</span>
          </div>
          <div class="fc-timer-controls">
            <button class="fc-btn" data-action="timer-toggle">${s.timer_running ? 'Pause' : 'Play'}</button>
            <button class="fc-btn" data-action="timer-reset">Reset</button>
            <button class="fc-btn" data-action="half-toggle">${Number(s.half) === 2 ? '1st Half' : '2nd Half'}</button>
          </div>
          <div class="fc-set-time">
            <input class="fc-set-input" id="fc-set-min" type="number" min="0" max="999" value="0" placeholder="min" />
            <span style="color:var(--text-3)">:</span>
            <input class="fc-set-input" id="fc-set-sec" type="number" min="0" max="59" value="0" placeholder="sec" />
            <button class="fc-btn" data-action="timer-set">Set</button>
          </div>
        </div>

        <!-- Events -->
        <div class="fc-section">
          <div class="fc-section-title">Events</div>
          <div class="fc-events">
            <button class="fc-btn fc-btn-goal" data-action="event-goal">Goal</button>
            <button class="fc-btn fc-btn-yellow" data-action="event-yellow">Yellow Card</button>
            <button class="fc-btn fc-btn-red" data-action="event-red">Red Card</button>
            <button class="fc-btn fc-btn-lineup" data-action="show-lineup">Lineup</button>
          </div>
          <div class="fc-input-row" id="fc-event-form" style="display:none;">
            <select class="fc-select" id="fc-event-team">
              <option value="home">${this._esc(s.home_team_name || 'Home')}</option>
              <option value="away">${this._esc(s.away_team_name || 'Away')}</option>
            </select>
            <input class="fc-input" id="fc-event-number" type="number" min="1" placeholder="#" />
            <button class="fc-btn" id="fc-event-confirm">Confirm</button>
            <button class="fc-btn" id="fc-event-cancel">Cancel</button>
          </div>
        </div>
      </div>
    `;
    this._bindEvents();
  }

  _bindEvents() {
    let pendingEventType = null;

    this.addEventListener('click', async (e) => {
      const btn = e.target.closest('[data-action]');
      if (!btn) return;
      const action = btn.dataset.action;

      if (action === 'score') {
        const team = btn.dataset.team;
        const delta = Number(btn.dataset.delta);
        const key = team === 'home' ? 'home_score' : 'away_score';
        const current = Number(this._state[key]) || 0;
        await this._sdk.setState({ [key]: Math.max(0, current + delta) });
      }

      if (action === 'timer-toggle') {
        if (this._state.timer_running) {
          // Pause
          const elapsed = this._getTimerSeconds();
          await this._sdk.setState({
            timer_running: false,
            timer_accumulated: elapsed,
            timer_start_epoch: null,
          });
        } else {
          // Play
          await this._sdk.setState({
            timer_running: true,
            timer_start_epoch: Math.floor(Date.now() / 1000),
          });
        }
      }

      if (action === 'timer-reset') {
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
        await this._sdk.setState(updates);
      }

      if (action === 'half-toggle') {
        const current = Number(this._state.half) || 1;
        await this._sdk.setState({ half: current === 1 ? 2 : 1 });
      }

      if (action === 'event-goal' || action === 'event-yellow' || action === 'event-red') {
        pendingEventType = action.replace('event-', '');
        const form = this.querySelector('#fc-event-form');
        if (form) form.style.display = 'flex';
      }

      if (action === 'show-lineup') {
        await this._sdk.triggerPopup('lineup');
      }
    });

    const confirmBtn = this.querySelector('#fc-event-confirm');
    const cancelBtn = this.querySelector('#fc-event-cancel');

    if (confirmBtn) {
      confirmBtn.addEventListener('click', async () => {
        const team = this.querySelector('#fc-event-team')?.value || 'home';
        const number = Number(this.querySelector('#fc-event-number')?.value) || 0;
        if (!number) return;

        // Look up player name
        const teamId = team === 'home' ? this._state.home_team_id : this._state.away_team_id;
        let playerName = `#${number}`;
        if (teamId) {
          try {
            const players = await this._sdk.queryData('players', { team_id: String(teamId), number: String(number) });
            if (players.length > 0) playerName = players[0].name;
          } catch { /* fallback to number */ }
        }

        const teamName = team === 'home' ? this._state.home_team_name : this._state.away_team_name;
        const teamColor = team === 'home' ? this._state.home_primary_color : this._state.away_primary_color;

        if (pendingEventType === 'goal') {
          // Increment score
          const key = team === 'home' ? 'home_score' : 'away_score';
          const current = Number(this._state[key]) || 0;
          await this._sdk.setState({ [key]: current + 1 });

          await this._sdk.triggerPopup('goal-popup', {
            player_name: playerName,
            player_number: String(number),
            team_name: teamName || team.toUpperCase(),
            team_color: teamColor || '#ffffff',
          });
          this._sdk.fireEvent('goal', { team, player_number: number, player_name: playerName });
        } else {
          const cardType = pendingEventType === 'yellow' ? 'Yellow Card' : 'Red Card';
          await this._sdk.triggerPopup('card-popup', {
            card_type: cardType,
            card_color: pendingEventType === 'yellow' ? '#eab308' : '#ef4444',
            player_name: playerName,
            player_number: String(number),
            team_name: teamName || team.toUpperCase(),
            team_color: teamColor || '#ffffff',
          });
          this._sdk.fireEvent(pendingEventType + '-card', { team, player_number: number, player_name: playerName });
        }

        // Hide form
        pendingEventType = null;
        const form = this.querySelector('#fc-event-form');
        if (form) form.style.display = 'none';
        const numInput = this.querySelector('#fc-event-number');
        if (numInput) numInput.value = '';
      });
    }

    if (cancelBtn) {
      cancelBtn.addEventListener('click', () => {
        pendingEventType = null;
        const form = this.querySelector('#fc-event-form');
        if (form) form.style.display = 'none';
      });
    }
  }

  _updateDisplay() {
    const s = this._state;
    const el = (sel) => this.querySelector(sel);
    const setText = (sel, text) => { const e = el(sel); if (e) e.textContent = text; };

    setText('.home-name', s.home_team_name || 'HOME');
    setText('.away-name', s.away_team_name || 'AWAY');
    setText('.home-score', String(Number(s.home_score) || 0));
    setText('.away-score', String(Number(s.away_score) || 0));
    setText('.fc-half', Number(s.half) === 2 ? '2nd Half' : '1st Half');

    const toggleBtn = this.querySelector('[data-action="timer-toggle"]');
    if (toggleBtn) toggleBtn.textContent = s.timer_running ? 'Pause' : 'Play';
    const halfBtn = this.querySelector('[data-action="half-toggle"]');
    if (halfBtn) halfBtn.textContent = Number(s.half) === 2 ? '1st Half' : '2nd Half';

    // Update team options in event form
    const teamSelect = el('#fc-event-team');
    if (teamSelect) {
      teamSelect.options[0].text = s.home_team_name || 'Home';
      teamSelect.options[1].text = s.away_team_name || 'Away';
    }
  }

  _esc(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
}

customElements.define('football-control', FootballControl);
export default FootballControl;
