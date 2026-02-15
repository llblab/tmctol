<script lang="ts">
  import { systemStore } from "$lib/entities/system/index.svelte";
  import { toFloat, fmt, fmtPrice } from "$lib/shared/format";

  const metrics = $derived.by(() => {
    const snap = systemStore.snapshot;
    if (!snap) return [];

    const supply = toFloat(snap.supply);
    const pXYK = snap.priceXyk ? toFloat(snap.priceXyk) : 0;
    const rN = toFloat(snap.reserveNative);
    const rF = toFloat(snap.reserveForeign);
    const burned = toFloat(snap.totalBurned);

    const history = systemStore.history;
    const pEffTMC =
      history.length > 0 ? history[history.length - 1].priceEffTMC : 0;
    const pRouter =
      history.length > 0 ? history[history.length - 1].priceRouter : null;
    const mcap = supply * pEffTMC;

    return [
      { label: "Supply", value: fmt(supply), color: "" },
      { label: "Market Cap", value: "$" + fmt(mcap), color: "" },
      {
        label: "Mint Price",
        value: "$" + fmtPrice(pEffTMC),
        color: "var(--mono-green)",
      },
      {
        label: "Router Price",
        value: pRouter != null ? "$" + fmtPrice(pRouter) : "—",
        color: "var(--mono-orange)",
      },
      {
        label: "XYK Price",
        value: snap.hasPool ? "$" + fmtPrice(pXYK) : "—",
        color: "var(--mono-purple)",
      },
      { label: "Pool Native", value: fmt(rN), color: "" },
      { label: "Pool Foreign", value: "$" + fmt(rF), color: "" },
      { label: "Burned", value: fmt(burned), color: "var(--mono-pink)" },
      {
        label: "Gravity Well",
        value: snap.gravityWellRatio.toFixed(1) + "%",
        color:
          snap.gravityWellRatio > 15
            ? "var(--mono-green)"
            : "var(--mono-orange)",
      },
    ];
  });
</script>

<div class="@container">
  <div class="grid grid-cols-2 @xs:grid-cols-3 @lg:grid-cols-3 gap-2">
    {#each metrics as m}
      <div
        class="bg-white rounded-xl border border-(--mono-border) px-3 py-2.5 shadow-[0_2px_8px_rgba(44,50,30,0.04)]"
      >
        <div class="text-[10px] text-(--mono-muted) uppercase tracking-wider">
          {m.label}
        </div>
        <div
          class="tabnum text-sm font-semibold truncate"
          style:color={m.color || "var(--mono-text)"}
        >
          {m.value}
        </div>
      </div>
    {/each}
  </div>
</div>
