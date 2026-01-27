class WasmAudioProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        this.wasm = undefined;
        this.sampleRate = undefined;
        this._samplesView;
        this._freqView;

        // Listen for WASM location and import it here
        this.port.onmessage = async (e) => {
            if (e.data.type === "sampleRate") {
                this.sampleRate = e.data.rate;
                if (this.wasm) {
                    this.wasm.exports.set_sample_rate(this.sampleRate);
                }
            }

            if (e.data.type === "wasm") {
                // Instantiate the module
                this.wasm = await WebAssembly.instantiate(e.data.module, {
                    env: {
                        memory: new WebAssembly.Memory({ initial: 1024 }),
                    },
                });

                // Then initialise our buffer views
                this._samplesView = this.createSampleInBufferView();
                this._freqView = this.createFreqOutBufferView();

                // And set the sample rate (if we have it)
                if (this.sampleRate !== undefined) {
                    this.wasm.exports.set_sample_rate(this.sampleRate);
                }
            }
        };
    }

    createSampleInBufferView() {
        const ptr = this.wasm.exports.sample_in_ptr();
        return new Float32Array(this.wasm.exports.memory.buffer, ptr, 128);
    }

    createFreqOutBufferView() {
        const ptr = this.wasm.exports.freq_out_ptr();
        const len = this.wasm.exports.freq_out_len();
        return new Float32Array(this.wasm.exports.memory.buffer, ptr, len);
    }

    get samplesView() {
        if (this._samplesView.buffer.detached)
            this._samplesView = this.createSampleInBufferView();
        return this._samplesView;
    }

    get freqView() {
        if (this._freqView.buffer.detached)
            this._freqView = this.createFreqOutBufferView();
        return this._freqView;
    }

    process(inputs) {
        // Must have init'd
        if (!this.wasm) return true;

        // TODO: maybe we need to allocate one buffer and re-use it from this side?
        // not sure how it works just passing an array buffer here..

        // Send through samples to be buffered wasm-side
        //   inputs will be a singleton (as determined at graph construction) with an array of channels of
        //   which each is a Float32Array w/ 128 samples
        //   e.g inputs[0][0] is the first channel of the first input, containing 128 audio f32 samples
        // inputs length and its channels length will vary depending on if stuff is hooked up?
        // TODO: do I need to think about channels here? Will it mono by default?
        const input = inputs?.[0]?.[0];
        if (input) {
            // Copy in the samples
            this.samplesView.set(input);

            // Process them
            this.wasm.exports.process_samples();

            // Read the out value
            // and send out of this node
            const freqs = Array.from(this.freqView);
            this.port.postMessage({
                type: "frequencies",
                freqs,
            });
        }

        return true;
    }
}

registerProcessor("wasm-audio-processor", WasmAudioProcessor);
