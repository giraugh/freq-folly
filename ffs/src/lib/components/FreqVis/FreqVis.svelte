<script lang="ts">
    let initiated = $state(false);
    let freqs = $state([]);
    async function initAudio() {
        initiated = true;

        // Setup and get an audio device
        const ctx = new AudioContext();
        const device = await navigator.mediaDevices.getUserMedia({
            audio: true,
        });

        // Register our worklet processor
        await ctx.audioWorklet.addModule("wasm-audio-processor.js");

        // Build the audio graph
        //    Our graph is just source->wasm-node w/ no outputs
        const source = ctx.createMediaStreamSource(device);
        const node = new AudioWorkletNode(ctx, "wasm-audio-processor");
        source.connect(node);

        // We compile the WASM here but do not instantiate
        const wasmBytes = await (await fetch("/ffaa.wasm")).arrayBuffer();
        const wasmModule = await WebAssembly.compile(wasmBytes);
        node.port.postMessage({
            type: "wasm",
            module: wasmModule,
        });

        node.port.onmessage = (e) => {
            freqs = e.data.freqs;
        };
    }
</script>

{#if !initiated}
    <button onclick={() => initAudio()}> Start </button>
{/if}

<div class="freqs">
    {#each freqs as freq, i (i)}
        <div style:height={`${5 + freq * 20}px`}></div>
    {/each}
</div>

<style>
    .freqs {
        display: grid;
        grid-template-columns: repeat(128, 1fr);
        gap: 0px;
        flex-direction: row;
        align-items: end;
        height: calc(100dvh - 20px); /* TEMP */
        width: 100vdw; /* TEMP */
        overflow-y: hidden;
    }

    .freqs > * {
        background: coral;
    }
</style>
