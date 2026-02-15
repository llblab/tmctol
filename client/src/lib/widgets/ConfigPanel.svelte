<script lang="ts">
  import { X } from "@lucide/svelte";

  import { systemStore } from "$lib/entities/system/index.svelte";
  import { fmt, toBigInt } from "$lib/shared/format";
  import { Button, NumberInput } from "$lib/shared/ui";

  type Props = {
    open: boolean;
    onclose: () => void;
  };

  let { open = $bindable(), onclose }: Props = $props();
  let faucetAmount = $state(10000);

  function apply() {
    onclose();
    systemStore.resetSimulation();
  }

  function drip() {
    if (faucetAmount <= 0) return;
    systemStore.depositForeign(toBigInt(faucetAmount));
    systemStore.addLog(`FAUCET +${fmt(faucetAmount)} USDC`, "info");
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="absolute inset-0 bg-black/35 backdrop-blur-[1px]"
      onclick={onclose}
    ></div>
    <div
      class="absolute right-0 top-0 bottom-0 w-80 bg-white border-l border-(--mono-border) p-5 flex flex-col gap-5 overflow-y-auto shadow-2xl"
    >
      <div class="flex items-center justify-between">
        <span class="text-sm font-semibold text-(--mono-text)"
          >Debug Settings</span
        >
        <button
          onclick={onclose}
          class="text-(--mono-muted) hover:text-(--mono-text) p-1 rounded hover:bg-(--mono-bg) transition-colors"
        >
          <X size={16} />
        </button>
      </div>

      <div class="flex flex-col gap-3">
        <span
          class="text-[10px] text-(--mono-muted) uppercase tracking-wider font-medium"
          >Initialization</span
        >
        <label class="block">
          <span class="text-xs text-(--mono-muted)">Initial Price</span>
          <NumberInput
            bind:value={systemStore.configValues.priceInitial}
            step={0.001}
            min={0.0001}
            class="w-full mt-1"
          />
        </label>
        <label class="block">
          <span class="text-xs text-(--mono-muted)">Slope</span>
          <NumberInput
            bind:value={systemStore.configValues.slope}
            step={0.0001}
            min={0}
            class="w-full mt-1"
          />
        </label>
        <label class="block">
          <span class="text-xs text-(--mono-muted)">Router Fee (‰)</span>
          <NumberInput
            bind:value={systemStore.configValues.routerFeePer1000}
            step={1}
            min={0}
            max={100}
            class="w-full mt-1"
          />
        </label>
        <label class="block">
          <span class="text-xs text-(--mono-muted)">User Share (PPM)</span>
          <NumberInput
            bind:value={systemStore.configValues.userSharePpm}
            step={1000}
            min={1}
            max={999999}
            class="w-full mt-1"
          />
        </label>
        <label class="block">
          <span class="text-xs text-(--mono-muted)"
            >Initial Foreign Balance</span
          >
          <NumberInput
            bind:value={systemStore.configValues.initialForeignBalance}
            step={10000}
            min={0}
            class="w-full mt-1"
          />
        </label>
        <button
          onclick={apply}
          class="w-full bg-(--mono-purple) hover:opacity-90 text-white text-sm font-medium py-2.5 rounded-xl transition-opacity"
        >
          Apply & Reset
        </button>
      </div>

      <!-- Faucet -->
      <div class="border-t border-(--mono-border) pt-4 flex flex-col gap-3">
        <span
          class="text-[10px] text-(--mono-muted) uppercase tracking-wider font-medium"
          >Testnet Faucet</span
        >
        <div class="flex gap-2">
          <NumberInput
            bind:value={faucetAmount}
            min={1}
            step={1000}
            class="flex-1 min-w-0"
          />
          <Button variant="secondary" onclick={drip}>Drip USDC</Button>
        </div>
      </div>
    </div>
  </div>
{/if}
