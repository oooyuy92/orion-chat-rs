<script lang="ts">
	import { SidebarFooter, SidebarMenu, SidebarMenuItem, SidebarMenuButton } from '$lib/components/ui/sidebar';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import { Avatar, AvatarFallback } from '$lib/components/ui/avatar';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import { goto } from '$app/navigation';
</script>

<SidebarFooter>
	<SidebarMenu>
		<SidebarMenuItem>
			<DropdownMenu>
				<DropdownMenuTrigger>
					{#snippet child({ props })}
						<SidebarMenuButton {...props} size="lg" class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground">
							<Avatar class="h-8 w-8 rounded-lg">
								<AvatarFallback class="rounded-lg">U</AvatarFallback>
							</Avatar>
							<div class="grid flex-1 text-left text-sm leading-tight">
								<span class="truncate font-semibold">User</span>
								<span class="truncate text-xs">user@example.com</span>
							</div>
							<ChevronUpIcon class="ml-auto size-4" />
						</SidebarMenuButton>
					{/snippet}
				</DropdownMenuTrigger>
				<DropdownMenuContent side="top" align="end" class="w-[--radix-dropdown-menu-trigger-width] min-w-56">
					<DropdownMenuItem onclick={() => goto('/settings')}>
						<SettingsIcon class="mr-2 h-4 w-4" />
						<span>Settings</span>
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</SidebarMenuItem>
	</SidebarMenu>
</SidebarFooter>
