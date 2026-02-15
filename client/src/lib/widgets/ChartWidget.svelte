<script lang="ts">
  import * as d3 from "d3";
  import { onDestroy, onMount } from "svelte";

  import { systemStore } from "$lib/entities/system/index.svelte";
  import { fmt, fmtPrice } from "$lib/shared/format";

  let containerEl: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let resizeObserver: ResizeObserver | null = null;
  let hidden = $state(new Set<string>());

  function toggleSeries(label: string) {
    const next = new Set(hidden);
    if (next.has(label)) next.delete(label);
    else next.add(label);
    hidden = next;
  }

  const CHART_RANGE = 80;
  const MARGIN = { top: 0, right: 32, bottom: 24, left: 32 };

  const LEGEND_ITEMS = [
    { label: "TMC", color: "#a6e22e" },
    { label: "Router", color: "#fd971f" },
    { label: "XYK", color: "#ae81ff" },
    { label: "Supply", color: "#66d9ef" },
  ];

  const COLORS = {
    xyk: "#8c63f4",
    tmc: "#8abf0f",
    router: "#f5861f",
    supply: "#2bb6cc",
  };

  const chartData = $derived.by(() => {
    const h = systemStore.history;
    return h.slice(Math.max(0, h.length - CHART_RANGE));
  });

  function render() {
    if (!svgEl || !containerEl || chartData.length === 0) return;

    const rect = containerEl.getBoundingClientRect();
    const width = rect.width;
    const height = rect.height;

    const svg = d3.select(svgEl).attr("width", width).attr("height", height);

    svg.selectAll("*").remove();

    const w = width - MARGIN.left - MARGIN.right;
    const h = height - MARGIN.top - MARGIN.bottom;
    if (w <= 0 || h <= 0) return;

    const g = svg
      .append("g")
      .attr("transform", `translate(${MARGIN.left},${MARGIN.top})`);

    // X scale
    const xExtent = d3.extent(chartData, (d) => d.step) as [number, number];
    const xScale = d3.scaleLinear().domain(xExtent).range([0, w]);

    // Y scale (prices)
    const allPrices = chartData.flatMap((d) =>
      [d.priceXYK, d.priceEffTMC, d.priceRouter].filter(
        (v): v is number => v != null && v > 0,
      ),
    );
    const yMax = d3.max(allPrices) ?? 1;
    const yScale = d3
      .scaleLinear()
      .domain([0, yMax * 1.05])
      .range([h, 0]);

    // Y1 scale (supply)
    const supplyMax = d3.max(chartData, (d) => d.supply) ?? 1;
    const y1Scale = d3
      .scaleLinear()
      .domain([0, supplyMax * 1.05])
      .range([h, 0]);

    // Grid
    const gridLines = yScale.ticks(5);
    g.append("g")
      .selectAll("line")
      .data(gridLines)
      .join("line")
      .attr("x1", 0)
      .attr("x2", w)
      .attr("y1", (d) => yScale(d))
      .attr("y2", (d) => yScale(d))
      .attr("stroke", "#d9dcc7")
      .attr("stroke-opacity", 0.5);

    // Supply area + line
    const supplyArea = d3
      .area<(typeof chartData)[0]>()
      .x((d) => xScale(d.step))
      .y0(h)
      .y1((d) => y1Scale(d.supply))
      .curve(d3.curveMonotoneX);

    const supplyLine = d3
      .line<(typeof chartData)[0]>()
      .x((d) => xScale(d.step))
      .y((d) => y1Scale(d.supply))
      .curve(d3.curveMonotoneX);

    // Supply area + line
    if (!hidden.has("Supply")) {
      g.append("path")
        .datum(chartData)
        .attr("d", supplyArea)
        .attr("fill", COLORS.supply)
        .attr("fill-opacity", 0.1);

      g.append("path")
        .datum(chartData)
        .attr("d", supplyLine)
        .attr("fill", "none")
        .attr("stroke", COLORS.supply)
        .attr("stroke-opacity", 1)
        .attr("stroke-width", 1);
    }

    // Price line generator
    function priceLine(accessor: (d: (typeof chartData)[0]) => number | null) {
      return d3
        .line<(typeof chartData)[0]>()
        .defined((d) => {
          const v = accessor(d);
          return v != null && v > 0;
        })
        .x((d) => xScale(d.step))
        .y((d) => yScale(accessor(d)!))
        .curve(d3.curveMonotoneX);
    }

    // XYK line
    if (!hidden.has("XYK")) {
      g.append("path")
        .datum(chartData)
        .attr(
          "d",
          priceLine((d) => (d.priceXYK > 0 ? d.priceXYK : null)),
        )
        .attr("fill", "none")
        .attr("stroke", COLORS.xyk)
        .attr("stroke-width", 1.5);
    }

    // TMC line
    if (!hidden.has("TMC")) {
      g.append("path")
        .datum(chartData)
        .attr(
          "d",
          priceLine((d) => d.priceEffTMC),
        )
        .attr("fill", "none")
        .attr("stroke", COLORS.tmc)
        .attr("stroke-width", 1.5);
    }

    // Router line + dots
    if (!hidden.has("Router")) {
      g.append("path")
        .datum(chartData)
        .attr(
          "d",
          priceLine((d) => d.priceRouter),
        )
        .attr("fill", "none")
        .attr("stroke", COLORS.router)
        .attr("stroke-width", 2);

      g.selectAll(".router-dot")
        .data(
          chartData.filter((d) => d.priceRouter != null && d.priceRouter > 0),
        )
        .join("circle")
        .attr("cx", (d) => xScale(d.step))
        .attr("cy", (d) => yScale(d.priceRouter!))
        .attr("r", 1.5)
        .attr("fill", COLORS.router);
    }

    // Left Y axis (price)
    g.append("g")
      .call(
        d3
          .axisLeft(yScale)
          .ticks(5)
          .tickSize(0)
          .tickFormat((v) => (+v).toFixed(2)),
      )
      .call((a) => a.select(".domain").remove())
      .call((a) =>
        a.selectAll(".tick text").attr("fill", "#6f7260").attr("font-size", 9),
      );

    // Right Y axis (supply)
    const fmtSupply = (v: d3.NumberValue) => {
      const n = +v;
      if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
      if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
      if (n >= 1e3) return (n / 1e3).toFixed(0) + "K";
      return n.toFixed(0);
    };

    g.append("g")
      .attr("transform", `translate(${w},0)`)
      .call(d3.axisRight(y1Scale).ticks(4).tickSize(0).tickFormat(fmtSupply))
      .call((a) => a.select(".domain").remove())
      .call((a) =>
        a
          .selectAll(".tick text")
          .attr("fill", COLORS.supply)
          .attr("fill-opacity", 0.4)
          .attr("font-size", 9),
      );

    // X axis (step labels)
    g.append("g")
      .attr("transform", `translate(0,${h})`)
      .call(
        d3
          .axisBottom(xScale)
          .ticks(Math.min(chartData.length, 6))
          .tickSize(0)
          .tickFormat((v) => `#${Math.round(+v)}`),
      )
      .call((a) => a.select(".domain").remove())
      .call((a) =>
        a
          .selectAll(".tick text")
          .attr("fill", "#6f7260")
          .attr("font-size", 9)
          .attr("dy", 8),
      );

    // Tooltip
    const tooltipEl = containerEl.querySelector(".d3-tooltip") as HTMLElement;
    const crosshair = g
      .append("line")
      .attr("y1", 0)
      .attr("y2", h)
      .attr("stroke", "#6f7260")
      .attr("stroke-dasharray", "2,2")
      .attr("opacity", 0);

    const bisect = d3.bisector<(typeof chartData)[0], number>(
      (d) => d.step,
    ).center;

    g.append("rect")
      .attr("width", w)
      .attr("height", h)
      .attr("fill", "none")
      .attr("pointer-events", "all")
      .on("mousemove", (event: MouseEvent) => {
        const [mx, my] = d3.pointer(event);
        const step = xScale.invert(mx);
        const idx = bisect(chartData, step);
        const d = chartData[idx];
        if (!d) return;

        const x = xScale(d.step);
        crosshair.attr("x1", x).attr("x2", x).attr("opacity", 1);

        const tmc = {
          label: "TMC",
          value: d.priceEffTMC,
          color: COLORS.tmc,
          isPrice: true,
        };
        const xyk = {
          label: "XYK",
          value: d.priceXYK,
          color: COLORS.xyk,
          isPrice: true,
        };
        const router = {
          label: "Router",
          value: d.priceRouter,
          color: COLORS.router,
          isPrice: true,
        };
        const supply = {
          label: "Supply",
          value: d.supply,
          color: COLORS.supply,
          isPrice: false,
        };

        const ceiling = (tmc.value ?? 0) >= (xyk.value ?? 0) ? tmc : xyk;
        const floor = ceiling === tmc ? xyk : tmc;
        const items = [ceiling, router, floor, supply].filter(
          (i) => i.value != null && !hidden.has(i.label),
        );

        if (tooltipEl) {
          tooltipEl.style.opacity = "1";
          const tooltipX = MARGIN.left + x + 12;
          const clampX = Math.min(tooltipX, width - 160);
          const tooltipY = MARGIN.top + my;
          const clampY = Math.min(Math.max(tooltipY, 4), height - 100);
          tooltipEl.style.left = `${clampX}px`;
          tooltipEl.style.top = `${clampY}px`;
          tooltipEl.innerHTML =
            `<div class="text-(--mono-text) mb-1">Step ${d.step}</div>` +
            items
              .map(
                (i) =>
                  `<div class="flex items-center gap-1.5">` +
                  `<span style="background:${i.color}" class="w-1.5 h-1.5 rounded-full inline-block shrink-0"></span>` +
                  `<span class="text-(--mono-muted)">${i.label}</span>` +
                  `<span class="text-(--mono-text) ml-auto tabnum">${i.isPrice ? "$" + fmtPrice(i.value!) : fmt(i.value!)}</span>` +
                  `</div>`,
              )
              .join("");
        }
      })
      .on("mouseleave", () => {
        crosshair.attr("opacity", 0);
        if (tooltipEl) tooltipEl.style.opacity = "0";
      });
  }

  $effect(() => {
    chartData;
    hidden;
    render();
  });

  onMount(() => {
    resizeObserver = new ResizeObserver(() => render());
    if (containerEl) resizeObserver.observe(containerEl);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
  });
</script>

<div class="flex flex-col h-full w-full min-h-0">
  <div class="flex items-center justify-center gap-4 shrink-0 py-1">
    {#each LEGEND_ITEMS as item}
      <button
        onclick={() => toggleSeries(item.label)}
        class={[
          "flex items-center gap-1 transition-opacity",
          hidden.has(item.label) && "opacity-30",
        ]}
      >
        <span
          class="w-2 h-2 mb-0.5 rounded-full inline-block"
          style:background={item.color}
        ></span>
        <span
          class={[
            "text-[10px]",
            hidden.has(item.label)
              ? "text-(--mono-border) line-through"
              : "text-(--mono-muted)",
          ]}>{item.label}</span
        >
      </button>
    {/each}
  </div>
  <div bind:this={containerEl} class="flex-1 min-h-0 relative">
    <svg bind:this={svgEl} class="block"></svg>
    <div
      class="d3-tooltip absolute pointer-events-none bg-white/95 border border-(--mono-border) rounded-lg px-2.5 py-2 text-[11px] opacity-0 transition-opacity z-10 min-w-35 backdrop-blur-sm shadow-sm"
    ></div>
  </div>
</div>
