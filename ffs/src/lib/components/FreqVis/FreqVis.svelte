<script lang="ts">
    const GRAVITY = 0.1;
    const BUOYANT_FORCE = 0.5;

    let initiated = $state(false);
    let spectrum = $state<number[]>([]);
    let svgEl = $state<SVGElement | undefined>(undefined);

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
            for (let i = 0; i < e.data.freqs.length; i++) {
                // Spectral dampening
                spectrum[i] = lerp(spectrum?.[i] ?? 0, e.data.freqs[i], 0.2);

                // Waviness
                spectrum[i] +=
                    Math.sin(
                        Date.now() / 500 + (30 * i) / e.data.freqs.length,
                    ) * 0.03;
            }
        };
    }

    // How big is the svg?
    const canvasSize = $derived(svgEl?.getBoundingClientRect());
    const canvasWidth = 1400; // $derived(canvasSize?.width ?? 0);
    const canvasHeight = 1200; //$derived(canvasSize?.height ?? 0);
    const spectrumScale = 70;
    const waterLine = $derived(canvasHeight * 0.9);

    // Derive line segments from the bar heights
    const segments = $derived.by(() => {
        // Apply smoothing to the spectrum
        const smoothedSpectrum = spectrum.map((v, i) => {
            const before = i > 0 ? spectrum[i - 1] : v;
            const after = i < spectrum.length - 1 ? spectrum[i + 1] : v;
            return (before + v + after) / 3;
        });

        let segments = [];
        for (let i = 1; i < spectrum.length; i++) {
            // Define line segment
            const prev = smoothedSpectrum[i - 1];
            const bar = smoothedSpectrum?.[i];
            const x1 = ((i - 1) / (spectrum.length - 1)) * canvasWidth;
            const x2 = (i / (spectrum.length - 1)) * canvasWidth;
            const y1 = waterLine - prev * spectrumScale;
            const y2 = waterLine - bar * spectrumScale;
            const xd = x2 - x1;
            const yd = y2 - y1;
            const dd = Math.sqrt(xd * xd + yd * yd);

            // normal
            const oxd = yd;
            const oyd = -xd; // double check this
            const nxd = oxd / dd;
            const nyd = oyd / dd;

            // add segment
            segments.push({
                x1,
                x2,
                y1,
                y2,
                xd,
                yd,
                dd,
                nxd,
                nyd,
            });
        }

        return segments;
    });

    // Create path data from segments
    const pathData = $derived.by(() => {
        if (segments.length === 0) return "";
        const start = `M${segments[0].x1}, ${segments[0].y1}`;
        const lineOps = segments.map((s) => `L${s.x2}, ${s.y2}`).join(" ");
        const returnLine = `L${canvasWidth + 20}, ${segments.at(-1)?.y2} L${canvasWidth + 20}, ${canvasHeight + 20} L${-5}, ${canvasHeight + 20}`;
        return `${start} ${lineOps} ${returnLine}`;
    });

    /** Describe the state of a single ball for now. Its positions are in px space ig */
    let ball = $state({ x: 700, y: -50, vx: 0, vy: 0, r: 30 });
    let goal = $state<{
        x: number;
        y: number;
        r: number;
        disabledAt: null | number;
    }>({ x: 700, y: 500, r: 350, disabledAt: null });
    let score = $state(0);

    function loop() {
        update();
        requestAnimationFrame(() => loop());
    }

    function update() {
        ball.vy += GRAVITY;

        ball.vy *= 0.995;
        ball.vx *= 0.995;

        ball.x += ball.vx;
        ball.y += ball.vy;

        // Is the ball in the goal?
        if (goal.disabledAt === null) {
            const goalXd = goal.x - ball.x;
            const goalYd = goal.y - ball.y;
            const goalD = Math.sqrt(goalXd * goalXd + goalYd * goalYd);
            if (goalD < Math.max(ball.r, goal.r)) {
                goal.r = Math.max(50, goal.r - 10);
                goal.x = Math.random() * canvasWidth;
                goal.y = 300 + 400 * Math.random();
                goal.disabledAt = Date.now();
                score += 1;
            }
        } else {
            const duration = Date.now() - goal.disabledAt;
            if (duration > 1_000) {
                goal.disabledAt = null;
            }
        }

        // Bounce of edges
        if (ball.x < 0) {
            ball.x = 0;
            ball.vx *= -1;
        }
        if (ball.x > canvasWidth) {
            ball.x = canvasWidth;
            ball.vx *= -1;
        }
        if (ball.y < 0) {
            ball.y = 0;
            ball.vy *= -1;
        }
        if (ball.y > canvasHeight) {
            ball.y = canvasHeight;
            ball.vy *= -1;
        }

        // Below water line?
        // Find a segment that matches our x position
        const activeSegment = segments.find((s) => {
            return ball.x >= s.x1 && ball.x <= s.x2;
        });
        if (activeSegment) {
            const s = activeSegment;
            const t = (ball.x - s.x1) / (s.x2 - s.x1);
            const surfaceY = lerp(s.y1, s.y2, t);
            if (ball.y > surfaceY) {
                // Apply bouyancy
                ball.vy -= BUOYANT_FORCE;
                ball.vx += s.nxd * BUOYANT_FORCE;

                // Apply damping
                //ball.vy *= 0.9;
                ball.vx *= 0.98;
            }
        }
    }

    function lerp(a: number, b: number, t: number): number {
        return a + (b - a) * t;
    }
</script>

{#if !initiated}
    <button
        class="start"
        onclick={() => {
            initAudio();
            loop();
        }}
    >
        Start
    </button>
{/if}

{#if score > 0}
    <h3>{score}</h3>
{/if}

<svg bind:this={svgEl} viewBox="0 0 {canvasWidth} {canvasHeight}">
    <circle cx={ball.x} cy={ball.y} r={ball.r} fill="coral" stroke="none" />
    <circle
        class="goal"
        class:disabled={goal.disabledAt !== null}
        cx={goal.x}
        cy={goal.y}
        r={goal.r}
        stroke="white"
        stroke-width="4"
        fill="#ffffff7a"
    />
    <path
        d={pathData}
        stroke="white"
        fill="#11a7e2"
        stroke-width="15"
        stroke-linecap="round"
    />
    <circle
        cx={ball.x}
        cy={ball.y}
        r={ball.r}
        stroke="white"
        stroke-width="3"
        fill="none"
    />
</svg>

<style>
    :global(body) {
        background: #031b2f;
    }

    .start {
        position: fixed;
        inset: 0;
        width: 150px;
        height: 80px;
        font-size: 2rem;
        margin: auto;

        background: white;
        border: none;
        color: #031b2f;
        font-family: sans-serif;
        padding: 10px;

        border-radius: 4px;
        font-weight: bold;
    }

    svg {
        background: linear-gradient(#114fe2, #11a7e2);
        width: min(1400px, 100%);
        position: absolute;
        inset: 0;
        margin: auto;
        z-index: -1;

        box-shadow: 0px 0px 50px 10px #0005;
        border-radius: 10px;
    }

    .goal {
        animation: pulse infinite linear 2s;
        transform-origin: center;

        &.disabled {
            opacity: 0.1;
        }
    }

    @keyframes pulse {
        from {
            scale: 1;
        }
        50% {
            scale: 0.98;
        }
        to {
            scale: 1;
        }
    }

    h3 {
        font-family: sans-serif;
        color: white;
        font-size: 3rem;
    }
</style>
