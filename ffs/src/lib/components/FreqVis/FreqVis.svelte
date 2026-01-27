<script lang="ts">
    let initiated = $state(false);

    const GRAVITY = 0.1;

    /** Describe the state of a single ball for now. Its positions are in px space ig */
    let ball = $state({ x: 200, y: 0, vx: 0, vy: 0, r: 30 });
    let bars = $state<Array<HTMLElement>>([]);

    /*

     TODO: instead of doing what im doing below, construct a curve from the heights and then determine the
           angle of the curve at the x position of the ball, then if its below the curve apply a force in that direction of the normal
           of the curve

    */

    function update() {
        ball.vy += GRAVITY;

        ball.vy *= 0.995;

        ball.x += ball.vx;
        ball.y += ball.vy;

        for (const bar of bars) {
            const rect = bar.getBoundingClientRect();
            const col = circleRectOverlap(ball, rect);
            if (col) {
                ball.x += (0.5 * col.xd) / Math.max(1, col.dd);
                ball.vy -= 0.3;
            }
        }

        if (ball.x < 0) {
            ball.x = 0;
            ball.vx *= -1;
        }

        if (ball.y < 0) {
            ball.y = 0;
            ball.vy *= -1;
        }
    }

    function minSegmentDist(start: number, end: number, point: number) {
        // Inside segment?
        if (start <= point && point <= end) {
            return 0;
        }

        // Otherwise distance to terminus
        return point < start ? start - point : point - end;
    }

    function circleRectOverlap(
        circle: { x: number; y: number; r: number },
        rect: DOMRect,
    ) {
        const xd = minSegmentDist(rect.left, rect.right, circle.x);
        const yd = minSegmentDist(rect.top, rect.bottom, circle.y);
        const dd = Math.sqrt(xd * xd + yd * yd);
        return dd > circle.r ? null : { xd, yd, dd };
    }

    function loop() {
        update();
        requestAnimationFrame(() => loop());
    }

    let spectrum = $state([]);
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
        node.port.postMessage({ type: "sampleRate", rate: ctx.sampleRate });
        source.connect(node);

        // We compile the WASM here but do not instantiate
        const wasmBytes = await (await fetch("/ffaa.wasm")).arrayBuffer();
        const wasmModule = await WebAssembly.compile(wasmBytes);
        node.port.postMessage({ type: "wasm", module: wasmModule });

        // Listen for frequency spectrum updates
        node.port.onmessage = (e) => {
            spectrum = e.data.freqs;
        };
    }
</script>

{#if !initiated}
    <button
        onclick={() => {
            initAudio();
            loop();
        }}
    >
        Start
    </button>
{/if}

<div class="freqs">
    {#each spectrum as height, i (i)}
        <div style:height={`${5 + height * 70}px`} bind:this={bars[i]}></div>
    {/each}
</div>

<div
    class="ball"
    style:left={`${ball.x}px`}
    style:top={`${ball.y}px`}
    style:width={`${ball.r * 2}px`}
></div>

<style>
    .freqs {
        display: grid;
        grid-template-columns: repeat(70, 1fr);
        gap: 0px;
        flex-direction: row;
        align-items: end;
        height: calc(100dvh); /* TEMP */
        width: 100vdw; /* TEMP */
        overflow-y: hidden;

        transition: height 0.1s;
    }

    .freqs > * {
        background: cornflowerblue;
    }

    .ball {
        position: absolute;
        background: coral;
        border-radius: 50%;
        aspect-ratio: 1;
    }
</style>
