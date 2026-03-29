<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import StatusDot from '$lib/components/StatusDot.svelte';
	import { socket, connected } from '$lib/api/socket';
	import { fetchStudios } from '$lib/api/api';
	import { getBackendUrl, getLocalIp } from '$lib/bridge';
	import type { Studio } from '$lib/types';
  import { IS_TAURI } from '$lib/bridge';
    import TitleBarWeb from '$lib/components/TitleBarWeb.svelte';

	let studios = $state<Studio[]>([]);
	let loading = $state(true);
	let studioPrograms = $state<Record<number, { name: string } | null>>({});
	let localIp = $state<string | null>(null);

	onMount(() => {
		fetchStudios().then((data) => {
			studios = data;
			loading = false;
		});

		// Poll until the LAN IP is resolved (initBackendUrl may still be in-flight)
		const ipInterval = setInterval(() => {
			const ip = getLocalIp();
			if (ip) {
				localIp = ip;
				clearInterval(ipInterval);
			}
		}, 200);
		setTimeout(() => clearInterval(ipInterval), 5000);

		socket.on('studio-state', (data: any) => {
			if (data.program) {
				studioPrograms = { ...studioPrograms, [data.studioId]: data.program };
			}
		});

		return () => {
			socket.off('studio-state');
		};
	});

	function selectStudio(studio: Studio) {
		goto(`/control?studio=${studio.id}`);
	}
</script>

<div class="page">
  <!-- Top bar -->
  {#if !IS_TAURI}
		<TitleBarWeb back={{ href: '/', label: 'Studios' }} />
	{/if}

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

  <footer class="ext-info">
    <span class="ext-label">External pages (OBS browser sources):</span>
    <code class="ext-url">{getBackendUrl()}/obs-background?studio=&#123;id&#125;</code>
    <span class="ext-sep">·</span>
    <code class="ext-url">{getBackendUrl()}/obs-overlay?studio=&#123;id&#125;</code>
    <span class="ext-sep">·</span>
    <span class="ext-note">hosted on port <strong>{new URL(getBackendUrl()).port || '80'}</strong></span>
    {#if localIp}
      <span class="ext-sep">·</span>
      <span class="ext-note">LAN IP: <strong>{localIp}</strong></span>
      <span class="ext-sep">·</span>
      <span class="ext-note lan-hint">access from other devices at <code class="ext-url">{localIp}:{new URL(getBackendUrl()).port || '80'}</code></span>
    {/if}
  </footer>
</div>

<style>
  .page {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
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

  /* External pages info footer */
  .ext-info {
    padding: 14px 32px;
    border-top: 1px solid var(--border-1);
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 12px;
    color: var(--text-3);
  }

  .ext-label {
    font-weight: 600;
    color: var(--text-2);
  }

  .ext-url {
    background: var(--surface-2);
    border: 1px solid var(--border-1);
    border-radius: var(--r-sm);
    padding: 2px 7px;
    font-size: 11px;
    color: var(--accent);
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  }

  .ext-sep {
    color: var(--border-2);
  }

  .ext-note strong {
    color: var(--text-2);
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
