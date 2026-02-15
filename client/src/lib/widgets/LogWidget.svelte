<script lang="ts">
  import { Trash2 } from "@lucide/svelte";

  import { systemStore } from "$lib/entities/system/index.svelte";

  const LOG_COLORS: Record<string, string> = {
    info: "text-(--mono-muted)",
    buy: "text-(--mono-green)",
    sell: "text-(--mono-pink)",
    error: "text-(--mono-orange)",
  };
</script>

<div class="flex flex-col h-full">
  <div
    class="flex items-center justify-between shrink-0 border-b border-(--mono-border) px-3 py-2"
  >
    <div class="flex items-center gap-2 text-xs text-(--mono-muted)">
      <span>Transaction Log</span>
      {#if systemStore.log.length > 0}
        <span
          class="text-[10px] bg-(--mono-border) text-(--mono-text) px-1.5 py-0.5 rounded-full tabnum"
        >
          {systemStore.log.length}
        </span>
      {/if}
    </div>
    <button
      onclick={() => systemStore.clearLog()}
      class="text-(--mono-muted) hover:text-(--mono-text) p-1 rounded hover:bg-(--mono-bg) transition-colors"
    >
      <Trash2 size={12} />
    </button>
  </div>

  <div
    class="flex-1 min-h-0 overflow-y-auto px-3 py-2 text-[11px] font-mono grid gap-0.5 content-start"
  >
    {#each systemStore.log as entry (entry.id)}
      <div class={LOG_COLORS[entry.type] || LOG_COLORS.info}>
        <span class="text-(--mono-border)">#{entry.step}</span>
        {entry.message}
      </div>
    {:else}
      <div class="text-(--mono-muted) py-2">No transactions yet</div>
    {/each}
  </div>
</div>
