<script lang="ts">
  import { systemStore } from "$lib/entities/system/index.svelte";
  import { toFloat, fmt } from "$lib/shared/format";

  const nBal = $derived(fmt(toFloat(systemStore.userBalance.native)));
  const fBal = $derived(fmt(toFloat(systemStore.userBalance.foreign)));
</script>

<div
  class="flex items-center gap-3 text-xs tabnum rounded-xl border border-(--mono-border) bg-white px-3 py-2 shadow-sm"
>
  <span class="text-(--mono-muted)">
    <span class="font-semibold text-(--mono-text)">{fBal}</span>
    USDC
  </span>
  <span class="text-(--mono-muted)">
    <span class="font-semibold text-(--mono-text)">{nBal}</span>
    NATIVE
  </span>
</div>
