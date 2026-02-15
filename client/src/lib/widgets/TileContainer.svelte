<script lang="ts">
  import type { TileNode } from "$lib/entities/layout/types";
  import TileContainer from "./TileContainer.svelte";
  import LeafPane from "./LeafPane.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";

  type Props = {
    node: TileNode;
  };

  let { node }: Props = $props();
</script>

{#if node.type === "leaf"}
  <LeafPane leaf={node} />
{:else}
  <div
    class={[
      "flex h-full w-full",
      node.direction === "horizontal" ? "flex-row" : "flex-col",
    ]}
  >
    <div
      style:flex="{node.ratio} 1 0%"
      style:min-width="0"
      style:min-height="0"
    >
      <TileContainer node={node.children[0]} />
    </div>
    <ResizeHandle splitId={node.id} direction={node.direction} />
    <div
      style:flex="{1 - node.ratio} 1 0%"
      style:min-width="0"
      style:min-height="0"
    >
      <TileContainer node={node.children[1]} />
    </div>
  </div>
{/if}
