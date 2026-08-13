// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import fs from 'fs';

// https://astro.build/config
export default defineConfig({
	site: 'https://zuygui.github.io/marrow/',
	base: '/marrow/',
	integrations: [
		starlight({
			title: 'Marrow Docs',
			customCss: ['./src/content/docs/styles/theme.css'],
			description: 'The official guide, CLI reference, and standard library docs for the Marrow programming language..',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/zuygui/marrow' }],
			editLink: {
				baseUrl: "https://github.com/zuygui/marrow/edit/docs/"
			},
			expressiveCode: {
        		shiki: {
          			langs: [JSON.parse(fs.readFileSync('./src/grammars/marrow.tmLanguage.json', 'utf-8')),],
       			},
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
						{ label: "FS", link: "/standard-library/fs" },
						{ label: "Map", link: "/standard-library/map" },
						{ label: "Sys", link: "/standard-library/sys" },
						{ label: "Vec", link: "/standard-library/vec" },
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
