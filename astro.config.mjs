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
						{ label: "Enums & Match", link: "/language/enums-and-match" },
						{ label: "Generics", link: "/language/generics" },
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
					label: "Guides",
					items: [
						{ label: "Async tasks", link: "/guides/async" },
						{ label: "Targets & cross-compilation", link: "/guides/cross-compilation" },
					]
				},
				{
					label: "Standard Library Reference",
					items: [
						{ label: "Overview", link: "/standard-library/overview" },
						{ label: "The platform layer", link: "/standard-library/platform" },
						{
							label: "Core",
							collapsed: true,
							items: [
								{ label: "IO", link: "/standard-library/io" },
								{ label: "Memory", link: "/standard-library/mem" },
								{ label: "Heap", link: "/standard-library/heap" },
								{ label: "Raw memory", link: "/standard-library/raw" },
								{ label: "String", link: "/standard-library/string" },
								{ label: "Text", link: "/standard-library/text" },
								{ label: "Fmt", link: "/standard-library/fmt" },
								{ label: "Vec", link: "/standard-library/vec" },
								{ label: "Map", link: "/standard-library/map" },
								{ label: "Sys", link: "/standard-library/sys" },
								{ label: "OS", link: "/standard-library/os" },
								{ label: "Env", link: "/standard-library/env" },
							]
						},
						{
							label: "Files & paths",
							collapsed: true,
							items: [
								{ label: "FS", link: "/standard-library/fs" },
								{ label: "Dir", link: "/standard-library/dir" },
								{ label: "Path", link: "/standard-library/path" },
							]
						},
						{
							label: "Math & time",
							collapsed: true,
							items: [
								{ label: "Math", link: "/standard-library/math" },
								{ label: "Rand", link: "/standard-library/rand" },
								{ label: "Time", link: "/standard-library/time" },
							]
						},
						{
							label: "Networking",
							collapsed: true,
							items: [
								{ label: "Net", link: "/standard-library/net" },
								{ label: "HTTP", link: "/standard-library/http" },
								{ label: "TLS", link: "/standard-library/tls" },
								{ label: "X.509", link: "/standard-library/x509" },
							]
						},
						{
							label: "Crypto & encoding",
							collapsed: true,
							items: [
								{ label: "Hash", link: "/standard-library/hash" },
								{ label: "ChaCha20-Poly1305", link: "/standard-library/chacha" },
								{ label: "AES-GCM", link: "/standard-library/aes" },
								{ label: "X25519", link: "/standard-library/x25519" },
								{ label: "BigInt", link: "/standard-library/bigint" },
								{ label: "RSA", link: "/standard-library/rsa" },
								{ label: "ECDSA", link: "/standard-library/ecdsa" },
								{ label: "ASN.1", link: "/standard-library/asn1" },
								{ label: "Encoding", link: "/standard-library/encoding" },
							]
						},
						{
							label: "Data formats",
							collapsed: true,
							items: [
								{ label: "JSON", link: "/standard-library/json" },
							]
						},
						{
							label: "Concurrency",
							collapsed: true,
							items: [
								{ label: "Task", link: "/standard-library/task" },
							]
						},
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
