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
	const syncWritingLanguage = async () => {
		if (linter && (await applyWritingLanguage(linter))) {
			// biome-ignore lint/correctness/noSelfAssign: reassigning makes Svelte pass the linter to the editor again, which lints the text with the new language.
			linter = linter;
		}
	};

	const onFocus = () => void syncWritingLanguage();
	window.addEventListener('focus', onFocus);

	return () => {
		window.removeEventListener('focus', onFocus);
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

/** Returns whether the language changed. */
async function applyWritingLanguage(target: Linter): Promise<boolean> {
	try {
		const writingLanguage = await Client.getHarperWritingLanguage();

		if ((await target.getWritingLanguage()) !== writingLanguage) {
			await target.setWritingLanguage(writingLanguage);
			return true;
		}
	} catch (error) {
		console.error('Unable to apply the writing language.', error);
	}

	return false;
}
</script>

<div class="h-screen w-screen">
	{#if linter}
		<Editor {linter} />
	{/if}
</div>
