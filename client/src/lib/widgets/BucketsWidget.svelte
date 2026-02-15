<script lang="ts">
  import { systemStore } from "$lib/entities/system/index.svelte";
  import { fmt, toFloat } from "$lib/shared/format";
  import type { BucketBalance } from "$lib/shared/types";
  import { Card } from "$lib/shared/ui";

  const COLORS: Record<string, string> = {
    a: "var(--mono-green)",
    b: "var(--mono-cyan)",
    c: "var(--mono-orange)",
    d: "var(--mono-pink)",
  };

  const LABELS: Record<string, string> = {
    a: "A (50%)",
    b: "B (16.7%)",
    c: "C (16.7%)",
    d: "D (16.6%)",
  };

  const bucketData = $derived.by(() => {
    const snap = systemStore.snapshot;
    if (!snap) return [];
    const totalLP = snap.supplyLp;
    const hasPool = snap.hasPool;

    return Array.from(snap.buckets.entries() as IterableIterator<[string, BucketBalance]>).map(([key, bucket]) => {
      const lp = toFloat(bucket.lp_tokens);
      const shareP =
        totalLP > 0n
          ? (Number((bucket.lp_tokens * 1000n) / totalLP) / 10).toFixed(1)
          : "0.0";

      let currentN = 0,
        currentF = 0;
      if (hasPool && totalLP > 0n && bucket.lp_tokens > 0n) {
        currentN = toFloat((snap.reserveNative * bucket.lp_tokens) / totalLP);
        currentF = toFloat((snap.reserveForeign * bucket.lp_tokens) / totalLP);
      }

      return {
        key,
        label: LABELS[key] || key.toUpperCase(),
        color: COLORS[key] || "var(--mono-text)",
        lp: fmt(lp),
        shareP,
        currentN: fmt(currentN),
        currentF: fmt(currentF),
      };
    });
  });

  const totalLP = $derived(
    systemStore.snapshot ? fmt(toFloat(systemStore.snapshot.supplyLp)) : "0",
  );
  const bufNative = $derived(
    systemStore.snapshot
      ? fmt(toFloat(systemStore.snapshot.bufferNative))
      : "0",
  );
  const bufForeign = $derived(
    systemStore.snapshot
      ? fmt(toFloat(systemStore.snapshot.bufferForeign))
      : "0",
  );
</script>

<div class="@container flex flex-col gap-3">
  <div class="grid grid-cols-2 @xs:grid-cols-2 @md:grid-cols-4 gap-2">
    {#each bucketData as b}
      <Card class="px-3 py-2.5 flex flex-col gap-1.5">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold" style:color={b.color}
            >{b.label}</span
          >
          <span class="text-[10px] text-(--mono-muted) tabnum">{b.shareP}%</span
          >
        </div>
        <div class="grid gap-0.5 text-[10px] text-(--mono-muted) tabnum">
          <div class="flex justify-between">
            <span>LP</span><span class="text-(--mono-text)">{b.lp}</span>
          </div>
          <div class="flex justify-between">
            <span>Native</span><span class="text-(--mono-text)"
              >{b.currentN}</span
            >
          </div>
          <div class="flex justify-between">
            <span>Foreign</span><span class="text-(--mono-text)"
              >{b.currentF}</span
            >
          </div>
        </div>
      </Card>
    {/each}
  </div>

  <div class="grid grid-cols-3 gap-2">
    <div
      class="bg-white rounded-xl border border-(--mono-border) px-3 py-2 shadow-[0_2px_8px_rgba(44,50,30,0.04)]"
    >
      <div class="text-[10px] text-(--mono-muted) uppercase tracking-wider">
        Total LP
      </div>
      <div class="tabnum text-sm font-semibold text-(--mono-text)">
        {totalLP}
      </div>
    </div>
    <div
      class="bg-white rounded-xl border border-(--mono-border) px-3 py-2 shadow-[0_2px_8px_rgba(44,50,30,0.04)]"
    >
      <div class="text-[10px] text-(--mono-muted) uppercase tracking-wider">
        Buffer ◆
      </div>
      <div class="tabnum text-sm font-medium text-(--mono-text)">
        {bufNative}
      </div>
    </div>
    <div
      class="bg-white rounded-xl border border-(--mono-border) px-3 py-2 shadow-[0_2px_8px_rgba(44,50,30,0.04)]"
    >
      <div class="text-[10px] text-(--mono-muted) uppercase tracking-wider">
        Buffer $
      </div>
      <div class="tabnum text-sm font-medium text-(--mono-text)">
        {bufForeign}
      </div>
    </div>
  </div>
</div>
