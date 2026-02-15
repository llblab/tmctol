<script lang="ts">
  import { Bug, RotateCcw, Sparkle } from "@lucide/svelte";
  import { onMount } from "svelte";

  import { layoutStore } from "$lib/entities/layout/index.svelte";
  import { systemStore } from "$lib/entities/system/index.svelte";
  import ConfigPanel from "$lib/widgets/ConfigPanel.svelte";
  import TileContainer from "$lib/widgets/TileContainer.svelte";
  import WalletWidget from "$lib/widgets/WalletWidget.svelte";

  let settingsOpen = $state(false);

  onMount(() => {
    systemStore.init();
    systemStore.addLog("Connected. Initial balance: 100,000 USDC", "info");
  });
</script>

<svelte:head>
  <title>TMCTOL DEX</title>
</svelte:head>

<div class="h-screen flex flex-col overflow-hidden">
  <header
    class="shrink-0 rounded-2xl border border-(--mono-border) bg-[linear-gradient(135deg,#ffffff_0%,#f2f8ec_46%,#edf6fa_100%)] shadow-[0_8px_32px_rgba(44,50,30,0.06)] m-3 mb-0"
  >
    <div class="flex items-center justify-between px-5 h-14">
      <div class="flex items-center gap-3">
        <h1
          class="inline-flex items-center gap-2 rounded-full bg-(--mono-bg) px-3 py-1 text-xs font-medium text-(--mono-muted)"
        >
          <Sparkle size={12} />
          TMCTOL Simulator
        </h1>
      </div>
      <div class="flex items-center gap-2">
        <WalletWidget />
        <button
          onclick={() => layoutStore.resetLayout()}
          class="inline-flex items-center gap-1 rounded-xl border border-(--mono-border) bg-white px-3 py-2 text-sm shadow-sm hover:border-(--mono-cyan)"
          title="Reset layout"
        >
          <RotateCcw size={14} />
        </button>
        <button
          onclick={() => (settingsOpen = true)}
          class="inline-flex items-center gap-1 rounded-xl bg-(--mono-purple) px-3 py-2 text-sm text-white shadow-sm hover:opacity-90"
          title="Debug Settings"
        >
          <Bug size={14} />
          Settings
        </button>
      </div>
    </div>
  </header>

  <ConfigPanel
    bind:open={settingsOpen}
    onclose={() => (settingsOpen = false)}
  />

  {#if systemStore.snapshot}
    <main class="flex-1 min-h-0 p-3 pt-3">
      <div class="h-full w-full">
        <TileContainer node={layoutStore.root} />
      </div>
    </main>
  {:else}
    <main class="flex-1 flex items-center justify-center p-3 pt-3">
      <div
        class="rounded-2xl border border-(--mono-border) bg-white/95 p-8 text-center shadow-[0_8px_32px_rgba(44,50,30,0.06)]"
      >
        <p class="text-(--mono-muted)">Initializing...</p>
      </div>
    </main>
  {/if}
</div>
