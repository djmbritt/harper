<script lang="ts">
import type { Linter } from 'harper.js';
import { Editor } from 'harper-editor';
import { onMount } from 'svelte';
import 'harper-editor/style.css';
import { Client } from '$lib/client';

let linter: Linter | null = null;

onMount(() => {
	void createLinter();

	// The language may have been changed in the settings window.
	const syncWritingLanguage = () => {
		if (linter) {
			void applyWritingLanguage(linter);
		}
	};

	window.addEventListener('focus', syncWritingLanguage);

	return () => {
		window.removeEventListener('focus', syncWritingLanguage);
	};
});

async function createLinter() {
	const [{ WorkerLinter }, { slimBinaryInlined }] = await Promise.all([
		import('harper.js'),
		import('harper.js/slimBinaryInlined'),
	]);

	const nextLinter = new WorkerLinter({ binary: slimBinaryInlined });
	await nextLinter.setup();
	await applyWritingLanguage(nextLinter);
	linter = nextLinter;
}

async function applyWritingLanguage(target: Linter) {
	try {
		const writingLanguage = await Client.getHarperWritingLanguage();

		if ((await target.getWritingLanguage()) !== writingLanguage) {
			await target.setWritingLanguage(writingLanguage);
		}
	} catch (error) {
		console.error('Unable to apply the writing language.', error);
	}
}
</script>

<div class="h-screen w-screen">
	{#if linter}
		<Editor {linter} />
	{/if}
</div>
