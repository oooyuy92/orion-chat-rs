<script lang="ts">
	import ProviderSettings from '$lib/components/settings/ProviderSettings.svelte';
	import { Button } from '$lib/components/ui/button';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import { i18n } from '$lib/stores/i18n.svelte';
</script>

<div class="flex h-full w-full flex-col">
	<header class="border-b px-6 py-4">
		<div class="flex items-center gap-4">
			<Button variant="ghost" size="icon" href="/">
				<ArrowLeftIcon class="h-5 w-5" />
			</Button>
			<h1 class="text-2xl font-semibold">{i18n.t.settingsPageTitle}</h1>
		</div>
	</header>

	<main class="flex-1 overflow-hidden">
		<ProviderSettings />
	</main>
</div>
