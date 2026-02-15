<script lang="ts">
  import { layoutStore } from "$lib/entities/layout/index.svelte";
  import type { DropEdge, PanelId, TileLeaf } from "$lib/entities/layout/types";
  import { PANEL_LABELS } from "$lib/entities/layout/types";
  import BucketsWidget from "./BucketsWidget.svelte";
  import ChartWidget from "./ChartWidget.svelte";
  import InfoPanel from "./InfoPanel.svelte";
  import LogWidget from "./LogWidget.svelte";
  import SwapWidget from "./SwapWidget.svelte";

  type Props = {
    leaf: TileLeaf;
  };
  let { leaf }: Props = $props();

  const ZONE_SIZE = 40;

  const isDragging = $derived(layoutStore.dragTab !== null);
  const canDropEdge = $derived(
    layoutStore.dragTab !== null &&
      !(layoutStore.dragTab.sourceLeafId === leaf.id && leaf.tabs.length <= 1),
  );

  let hoveredEdge = $state<DropEdge | null>(null);
  let insertIndicatorX = $state<number | null>(null);
  let tabBarEl = $state<HTMLDivElement>(null!);
  let containerEl: HTMLDivElement;

  function detectEdge(e: DragEvent): DropEdge | null {
    if (!containerEl || !tabBarEl) return null;
    const rect = containerEl.getBoundingClientRect();
    const tbh = tabBarEl.offsetHeight;
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (y < tbh) return null;

    const contentH = rect.height - tbh;
    const yInContent = y - tbh;

    const nearBottom = yInContent > contentH - ZONE_SIZE;
    const nearLeft = x < ZONE_SIZE;
    const nearRight = x > rect.width - ZONE_SIZE;

    if (+nearBottom + +nearLeft + +nearRight !== 1) return null;

    if (nearBottom) return "bottom";
    if (nearLeft) return "left";
    return "right";
  }

  function onTabDragStart(e: DragEvent, tabId: PanelId) {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", tabId);
    requestAnimationFrame(() => layoutStore.startDrag(tabId, leaf.id));
  }

  function onTabDragEnd() {
    layoutStore.endDrag();
    hoveredEdge = null;
    insertIndicatorX = null;
  }

  function onOverlayDragOver(e: DragEvent) {
    if (!canDropEdge) return;
    const edge = detectEdge(e);
    hoveredEdge = edge;
    if (edge) {
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    }
  }

  function onOverlayDragLeave() {
    hoveredEdge = null;
  }

  function onOverlayDrop(e: DragEvent) {
    const edge = detectEdge(e);
    hoveredEdge = null;
    if (!edge) return;
    e.preventDefault();
    layoutStore.dropOnEdge(leaf.id, edge);
  }

  function onTabBarDragOver(e: DragEvent) {
    if (!layoutStore.dragTab) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    updateInsertIndicator(e);
  }

  function onTabBarDragLeave() {
    insertIndicatorX = null;
  }

  function updateInsertIndicator(e: DragEvent) {
    if (!tabBarEl) {
      insertIndicatorX = null;
      return;
    }
    const barRect = tabBarEl.getBoundingClientRect();
    const buttons = tabBarEl.querySelectorAll<HTMLElement>("[data-tab-id]");
    const idx = computeInsertIndex(e);

    if (buttons.length === 0) {
      insertIndicatorX = 2;
      return;
    }

    if (idx >= buttons.length) {
      const last = buttons[buttons.length - 1].getBoundingClientRect();
      insertIndicatorX = last.right - barRect.left;
    } else {
      insertIndicatorX =
        buttons[idx].getBoundingClientRect().left - barRect.left;
    }
  }

  function onTabBarDrop(e: DragEvent) {
    e.preventDefault();
    insertIndicatorX = null;
    if (!layoutStore.dragTab) return;

    const { tabId, sourceLeafId } = layoutStore.dragTab;
    const index = computeInsertIndex(e);

    if (sourceLeafId === leaf.id) {
      layoutStore.reorderTab(leaf.id, tabId, index);
      layoutStore.endDrag();
    } else {
      layoutStore.dropOnTabBar(leaf.id, index);
    }
  }

  function computeInsertIndex(e: DragEvent): number {
    if (!tabBarEl) return leaf.tabs.length;
    const buttons = tabBarEl.querySelectorAll<HTMLElement>("[data-tab-id]");
    for (let i = 0; i < buttons.length; i++) {
      const rect = buttons[i].getBoundingClientRect();
      if (e.clientX < rect.left + rect.width / 2) return i;
    }
    return leaf.tabs.length;
  }
</script>

<div
  bind:this={containerEl}
  class="flex flex-col h-full w-full overflow-hidden relative rounded-2xl border border-(--mono-border) bg-white shadow-[0_2px_10px_rgba(44,50,30,0.06)]"
>
  <!-- Tab header (z-50 stays above drop overlay) -->
  <div
    bind:this={tabBarEl}
    ondragover={onTabBarDragOver}
    ondragleave={onTabBarDragLeave}
    ondrop={onTabBarDrop}
    class="flex px-1 items-center shrink-0 border-b justify-center gap-0.5 border-(--mono-border) bg-(--mono-bg) py-1 relative z-10"
    role="tablist"
    tabindex="-1"
  >
    {#each leaf.tabs as tabId (tabId)}
      <button
        data-tab-id={tabId}
        draggable="true"
        ondragstart={(e) => onTabDragStart(e, tabId)}
        ondragend={onTabDragEnd}
        onclick={() => layoutStore.setActiveTab(leaf.id, tabId)}
        role="tab"
        aria-selected={leaf.activeTab === tabId}
        class={[
          "px-3 py-1 rounded-lg text-xs font-medium select-none cursor-grab active:cursor-grabbing whitespace-nowrap transition-colors",
          leaf.activeTab === tabId
            ? "text-(--mono-purple) bg-(--mono-purple)/10"
            : "text-(--mono-muted) hover:text-(--mono-text) hover:bg-white",
        ]}
      >
        {PANEL_LABELS[tabId]}
      </button>
    {/each}
    {#if insertIndicatorX !== null && isDragging}
      <div
        class="absolute top-1 bottom-1 w-0.5 bg-(--mono-purple) rounded-full pointer-events-none"
        style:left="{insertIndicatorX}px"
      ></div>
    {/if}
  </div>

  <!-- Content -->
  <div
    class={[
      "flex-1 min-h-0 @container",
      leaf.activeTab === "chart" || leaf.activeTab === "log"
        ? "p-0"
        : "p-2 overflow-y-auto",
    ]}
  >
    {#if leaf.activeTab === "swap"}
      <SwapWidget />
    {:else if leaf.activeTab === "chart"}
      <ChartWidget />
    {:else if leaf.activeTab === "info"}
      <InfoPanel />
    {:else if leaf.activeTab === "buckets"}
      <BucketsWidget />
    {:else if leaf.activeTab === "log"}
      <LogWidget />
    {/if}
  </div>

  <!-- Drop overlay: offset below tab bar -->
  {#if isDragging && canDropEdge}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      role="none"
      class="absolute left-0 right-0 bottom-0 z-40"
      style:top="{tabBarEl?.offsetHeight ?? 24}px"
      ondragover={onOverlayDragOver}
      ondragleave={onOverlayDragLeave}
      ondrop={onOverlayDrop}
    >
      {#if hoveredEdge === "right"}
        <div
          class="absolute top-0 right-0 bottom-0 w-10 bg-(--mono-purple)/25 border-l-2 border-(--mono-purple) pointer-events-none"
        ></div>
      {:else if hoveredEdge === "bottom"}
        <div
          class="absolute bottom-0 left-0 right-0 h-10 bg-(--mono-purple)/25 border-t-2 border-(--mono-purple) pointer-events-none"
        ></div>
      {:else if hoveredEdge === "left"}
        <div
          class="absolute top-0 left-0 bottom-0 w-10 bg-(--mono-purple)/25 border-r-2 border-(--mono-purple) pointer-events-none"
        ></div>
      {/if}
    </div>
  {/if}
</div>
