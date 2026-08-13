// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'My Docs',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/withastro/starlight' }],
			editLink: {
				// branch 'docs' from 'zuygui/marrow' repo
				baseUrl: "https://github.com/withastro/starlight/edit/main/"
			},
			sidebar: [
				{
					label: "Getting Started",
					items: [
						{ label: "Introduction", link: "/getting-started/introduction" },
						{ label: "Installation", link: "/getting-started/installation" },
						{ label: "Your first Marrow Program", link: "/getting-started/your-first-marrow-program" },
					]
				},
				{
					label: "Language Reference",
					items: [
						{ label: "Overview", link: "/language/overview" },
						{ label: "Types", link: "/language/types" },
						{ label: "Variables", link: "/language/variables" },
						{ label: "Functions", link: "/language/functions" },
						{ label: "Control Flow", link: "/language/control-flow" },
						{ label: "Decorators & Modules", link: "/language/decorators-and-modules" },
						{ label: "Variadic Functions", link: "/language/variadic-functions" },
					]
				},
				{
					label: "CLI Reference",
					items: [
						{ label: "Overview", link: "/cli/overview" },
					]
				},
				{
					label: "Standard Library Reference",
					items: [
						{ label: "Overview", link: "/standard-library/overview" },
						{ label: "IO", link: "/standard-library/io" },
						{ label: "Mem", link: "/standard-library/mem" },
						{ label: "String", link: "/standard-library/string" },
					]
				}
			]
		}),
	],
	vite: {
		server: {
			watch: {
				// force watch to use polling (fixes issues with WSL2)
				usePolling: true,
			},
		},
	},
});
