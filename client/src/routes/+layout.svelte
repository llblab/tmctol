<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<svelte:head>
	<title>TMCTOL DEX</title>
</svelte:head>

{@render children()}
