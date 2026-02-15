<script lang="ts">
  import { layoutStore } from "$lib/entities/layout/index.svelte";

  type Props = {
    splitId: string;
    direction: "horizontal" | "vertical";
  };

  let { splitId, direction }: Props = $props();
  let handleEl: HTMLDivElement;
  let dragging = $state(false);

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    handleEl.setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const parent = handleEl.parentElement;
    if (!parent) return;

    const rect = parent.getBoundingClientRect();
    const ratio =
      direction === "horizontal"
        ? (e.clientX - rect.left) / rect.width
        : (e.clientY - rect.top) / rect.height;

    layoutStore.resizeSplit(splitId, ratio);
  }

  function onPointerUp() {
    dragging = false;
  }

  function onDblClick() {
    layoutStore.resizeSplit(splitId, 0.5);
  }
</script>

<div
  bind:this={handleEl}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  ondblclick={onDblClick}
  role="separator"
  class={[
    "shrink-0 z-10 bg-clip-content transition-colors bg-transparent hover:bg-(--mono-purple)/30 active:bg-(--mono-purple)/60",
    direction === "horizontal"
      ? "px-1 w-3 cursor-col-resize"
      : "py-1 h-3 cursor-row-resize",
  ]}
></div>
