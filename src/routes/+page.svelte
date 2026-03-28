<div class="page">
  <!-- Top bar -->
  <header class="topbar">
    <span class="brand">OBS Manager</span>
    <div class="topbar-right">
      <a class="topbar-link" href="/program-editor">Programs</a>
      <a class="topbar-link" href="/ad-editor">Ads</a>
      <a class="topbar-link" href="/studio-editor">Studios</a>
      <a class="topbar-link" href="/import-export">Import/Export</a>
      <div style="width: 10px;"></div>
      <StatusDot connected={$connected} />
    </div>
  </header>

  <!-- Main content -->
  <main class="content">
    <div class="page-title">
      <h2>Select a Studio to Control</h2>
    </div>

    {#if loading}
      <div class="empty-message">Loading studios…</div>
    {:else if studios.length === 0}
      <div class="empty-message">
        No studios found.
        <a href="/studio-editor">Create one →</a>
      </div>
    {:else}
      <div class="studio-grid">
        {#each studios as studio (studio.id)}
          {@const prog = studioPrograms[studio.id]}
          <button
            class="studio-card"
            class:has-program={!!prog}
            onclick={() => selectStudio(studio)}
          >
            <div class="studio-number">
              {studio.id.toString().padStart(2, "0")}
            </div>
            <div class="studio-body">
              <div class="studio-name">{studio.name}</div>
              <div class="studio-status">
                {#if prog}
                  <span class="status-dot live"></span>
                  <span class="status-text live">{prog.name}</span>
                {:else}
                  <span class="status-dot idle"></span>
                  <span class="status-text idle">No program</span>
                {/if}
              </div>
            </div>
            <div class="studio-arrow">
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  </main>
</div>

<style>
  .page {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  /* Top bar */
  .topbar {
    height: 50px;
    padding: 0 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-1);
    background: var(--surface-1);
    flex-shrink: 0;
  }

  .brand {
    font-size: 13px;
    font-weight: 800;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-1);
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .topbar-link {
    font-size: 16px;
    color: var(--text-3);
    padding: 4px 10px;
    border-radius: var(--r-sm);
    text-decoration: none;
    transition: all 0.15s;
  }

  .topbar-link:hover {
    color: var(--text-1);
    background: var(--surface-2);
  }

  /* Content */
  .content {
    flex: 1;
    padding: 40px 32px;
    max-width: 900px;
    margin: 0 auto;
    width: 100%;
  }

  .page-title {
    margin-bottom: 32px;
  }

  .page-title h2 {
    font-size: 20px;
    font-weight: 400;
    color: var(--text-3);
  }

  /* Grid */
  .studio-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 12px;
  }

  .studio-card {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 18px;
    background: var(--surface-1);
    border: 1px solid var(--border-1);
    border-radius: var(--r-lg);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
    font-family: inherit;
  }

  .studio-card:hover {
    border-color: var(--border-2);
    background: var(--surface-2);
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  }

  .studio-card.has-program {
    border-color: rgba(56, 189, 248, 0.25);
  }

  .studio-card.has-program:hover {
    border-color: var(--accent);
    box-shadow: 0 4px 20px rgba(56, 189, 248, 0.1);
  }

  .studio-number {
    font-size: 22px;
    font-weight: 800;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
    width: 36px;
  }

  .studio-body {
    flex: 1;
    min-width: 0;
  }

  .studio-name {
    font-size: clamp(0.8rem, 4vw, 2rem);
    font-weight: 700;
    color: var(--text-1);
    margin-bottom: 4px;
    line-height: 1.2;
    overflow-wrap: break-word;
  }

  .studio-status {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    margin-top: 0.4em; /* Scales with responsive text */
  }

  .status-dot.live {
    background: #ef3333;
    box-shadow: 0 0 6px #ef3333;
    animation: pulse 2s ease-in-out infinite;
  }

  .status-dot.idle {
    background: var(--border-2);
  }

  .status-text {
    font-size: clamp(0.65rem, 3.5vw, 1.125rem);
    line-height: 1.2;
    overflow-wrap: break-word;
  }

  .status-text.live {
    color: #cc6666;
  }
  .status-text.idle {
    color: var(--text-3);
  }

  .studio-arrow {
    color: var(--text-3);
    flex-shrink: 0;
    transition:
      transform 0.15s,
      color 0.15s;
  }

  .studio-card:hover .studio-arrow {
    transform: translateX(3px);
    color: var(--text-2);
  }

  .empty-message {
    padding: 48px 0;
    text-align: center;
    font-size: 13px;
    color: var(--text-3);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
</style>
