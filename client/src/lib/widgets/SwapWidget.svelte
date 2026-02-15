<script lang="ts">
  import { ArrowUpDown } from "@lucide/svelte";

  import { systemStore } from "$lib/entities/system/index.svelte";
  import {
    fmt,
    fmtOut,
    fmtPrice,
    toBigInt,
    toFloat,
  } from "$lib/shared/format";
  import { Badge, Card } from "$lib/shared/ui";

  let inputValue = $state("");

  let quote = $derived.by(() => {
    const raw = parseFloat(inputValue) || 0;
    if (raw <= 0 || !systemStore.snapshot) return null;
    try {
      const amount = toBigInt(raw);
      return systemStore.direction === "buy"
        ? systemStore.getQuoteBuy(amount)
        : systemStore.getQuoteSell(amount);
    } catch {
      return null;
    }
  });

  let buttonState = $derived.by(() => {
    const raw = parseFloat(inputValue) || 0;
    const isBuy = systemStore.direction === "buy";
    const bal = isBuy
      ? toFloat(systemStore.userBalance.foreign)
      : toFloat(systemStore.userBalance.native);

    if (raw <= 0) return { text: "Enter an amount", disabled: true };
    if (raw > bal)
      return {
        text: `Insufficient ${isBuy ? "USDC" : "NATIVE"} balance`,
        disabled: true,
      };
    if (!isBuy && !systemStore.snapshot?.hasPool)
      return { text: "No liquidity", disabled: true };
    return { text: "Swap", disabled: false };
  });

  const isBuy = $derived(systemStore.direction === "buy");
  const inBalance = $derived(
    isBuy ? systemStore.userBalance.foreign : systemStore.userBalance.native,
  );
  const outBalance = $derived(
    isBuy ? systemStore.userBalance.native : systemStore.userBalance.foreign,
  );

  function executeSwap() {
    const raw = parseFloat(inputValue);
    if (!raw || raw <= 0) return;
    try {
      if (isBuy) {
        const r = systemStore.buyNative(toBigInt(raw));
        systemStore.addLog(
          `BUY ${fmt(raw)} USDC → ${fmt(toFloat(r.native_out!))} NATIVE via ${r.route}`,
          "buy",
        );
      } else {
        const r = systemStore.sellNative(toBigInt(raw));
        systemStore.addLog(
          `SELL ${fmt(raw)} NATIVE → ${fmt(toFloat(r.foreign_out!))} USDC via XYK`,
          "sell",
        );
      }
      inputValue = "";
    } catch (e: any) {
      systemStore.addLog(e.message, "error");
    }
  }

  function flipTokens() {
    systemStore.flipDirection();
    inputValue = "";
  }

  const routeInfo = $derived.by(() => {
    const snap = systemStore.snapshot;
    if (!snap) return null;
    const history = systemStore.history;
    const last = history.length > 0 ? history[history.length - 1] : null;
    const tmcPrice = last?.priceEffTMC ?? 0;
    const xykPrice = last?.priceXYK ?? 0;
    const routerPrice = last?.priceRouter ?? null;
    const feePercent = (systemStore.configValues.routerFeePer1000 / 10).toFixed(
      1,
    );
    const bestRoute = xykPrice > 0 && xykPrice < tmcPrice ? "XYK" : "TMC";
    return { tmcPrice, xykPrice, routerPrice, feePercent, bestRoute };
  });

  function setMax() {
    const bal = toFloat(inBalance);
    inputValue = bal > 0 ? bal.toFixed(2) : "";
  }
</script>

<Card class="p-4 mx-auto max-w-100 flex flex-col gap-4">
  <!-- Token In -->
  <div
    class="rounded-xl border border-(--mono-border) bg-[linear-gradient(180deg,#f8faf2_0%,#fefefe_100%)] p-3"
  >
    <div class="flex items-center justify-between">
      <span class="text-xs text-(--mono-muted)">You pay</span>
      <button
        onclick={setMax}
        class="text-[10px] text-(--mono-purple) hover:underline tabnum"
      >
        Balance: {fmt(toFloat(inBalance))}
      </button>
    </div>
    <div class="mt-2 flex items-center gap-2">
      <input
        type="number"
        placeholder="0.00"
        min="0"
        bind:value={inputValue}
        class="flex-1 border-none bg-transparent text-xl font-semibold tabnum placeholder-(--mono-border) min-w-0 focus:outline-none"
      />
      <button
        onclick={flipTokens}
        class="shrink-0 flex items-center gap-1.5 rounded-xl border border-(--mono-border) bg-white pl-2 pr-3 py-1.5 text-sm font-medium hover:border-(--mono-purple) transition-colors"
      >
        <span
          class={[
            "w-5 h-5 rounded-full flex items-center justify-center text-[10px]",
            isBuy
              ? "bg-(--mono-orange)/20 text-(--mono-orange)"
              : "bg-(--mono-green)/20 text-(--mono-green)",
          ]}
        >
          {isBuy ? "$" : "◆"}
        </span>
        <span>{isBuy ? "USDC" : "NATIVE"}</span>
      </button>
    </div>
  </div>

  <!-- Flip -->
  <div class="flex justify-center -my-2 relative z-10">
    <button
      onclick={flipTokens}
      class="bg-white border-4 border-(--mono-surface) rounded-xl w-9 h-9 flex items-center justify-center text-(--mono-muted) hover:text-(--mono-purple) hover:rotate-180 transition-all duration-200 shadow-sm"
    >
      <ArrowUpDown size={16} />
    </button>
  </div>

  <!-- Token Out -->
  <div class="rounded-xl border border-(--mono-border) bg-white p-3">
    <div class="flex items-center justify-between">
      <span class="text-xs text-(--mono-muted)">You receive</span>
      <span class="text-[10px] text-(--mono-border) tabnum">
        Balance: {fmt(toFloat(outBalance))}
      </span>
    </div>
    <div class="mt-2 flex items-center gap-2">
      <div
        class="flex-1 text-xl font-semibold tabnum text-(--mono-muted) truncate min-w-0"
      >
        {quote ? fmtOut(toFloat(quote.out)) : "0.00"}
      </div>
      <div
        class="shrink-0 flex items-center gap-1.5 rounded-xl border border-(--mono-border) bg-(--mono-bg) pl-2 pr-3 py-1.5 text-sm font-medium"
      >
        <span
          class={[
            "w-5 h-5 rounded-full flex items-center justify-center text-[10px]",
            isBuy
              ? "bg-(--mono-green)/20 text-(--mono-green)"
              : "bg-(--mono-orange)/20 text-(--mono-orange)",
          ]}
        >
          {isBuy ? "◆" : "$"}
        </span>
        <span>{isBuy ? "NATIVE" : "USDC"}</span>
      </div>
    </div>
  </div>

  <!-- Route Info -->
  <div class="grid gap-1 text-xs text-(--mono-muted) px-1">
    <div class="flex justify-between">
      <span>Rate</span>
      <span class="text-(--mono-text) tabnum">
        {#if quote}
          ${fmtPrice(quote.effectivePrice)} per NATIVE
        {:else if routeInfo?.routerPrice}
          ${fmtPrice(routeInfo.routerPrice)} per NATIVE
        {:else}
          —
        {/if}
      </span>
    </div>
    <div class="flex justify-between">
      <span>Router fee</span>
      <span class="tabnum">
        {#if quote}
          {fmt(toFloat(quote.fee))} {isBuy ? "USDC" : "NATIVE"}
        {:else}
          {routeInfo?.feePercent ?? "0.5"}%
        {/if}
      </span>
    </div>
    <div class="flex justify-between">
      <span>Route</span>
      <span class="text-(--mono-text)">
        {#if quote}
          <Badge variant={quote.route === "TMC" ? "tmc" : "xyk"}
            >{quote.route}</Badge
          >
        {:else if routeInfo}
          <Badge variant={routeInfo.bestRoute === "TMC" ? "tmc" : "xyk"}
            >{routeInfo.bestRoute}</Badge
          >
        {:else}
          —
        {/if}
      </span>
    </div>
    <div class="flex justify-between">
      <span>Compare</span>
      <span class="tabnum">
        {#if quote && !quote.isSell && toFloat(quote.tmcOut) > 0 && toFloat(quote.xykOut) > 0}
          TMC {fmt(toFloat(quote.tmcOut))} · XYK {fmt(toFloat(quote.xykOut))}
        {:else if routeInfo}
          TMC ${fmtPrice(routeInfo.tmcPrice)}{routeInfo.xykPrice > 0
            ? ` · XYK $${fmtPrice(routeInfo.xykPrice)}`
            : ""}
        {:else}
          —
        {/if}
      </span>
    </div>
    {#if !quote && parseFloat(inputValue) > 0}
      {#if !isBuy && !systemStore.snapshot?.hasPool}
        <div class="text-(--mono-warn)">
          Pool not initialized. Buy via TMC first to create liquidity.
        </div>
      {:else}
        <div class="text-(--mono-warn)">
          No route available. Try a larger amount or mint first.
        </div>
      {/if}
    {/if}
  </div>

  <!-- Swap Button -->
  <button
    onclick={executeSwap}
    disabled={buttonState.disabled}
    class="w-full py-3 rounded-xl text-sm font-semibold text-white transition-opacity"
    style:background="var(--mono-purple)"
    style:opacity={buttonState.disabled ? 0.5 : 1}
  >
    {buttonState.text}
  </button>
</Card>
