<script lang="ts">
	import { SidebarFooter, SidebarMenu, SidebarMenuItem, SidebarMenuButton } from '$lib/components/ui/sidebar';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import { goto } from '$app/navigation';
</script>

<SidebarFooter>
	<SidebarMenu>
		<SidebarMenuItem>
			<SidebarMenuButton onclick={() => goto('/settings')} class="cursor-pointer">
				<SettingsIcon class="size-4" />
				<span>Settings</span>
			</SidebarMenuButton>
		</SidebarMenuItem>
	</SidebarMenu>
</SidebarFooter>
