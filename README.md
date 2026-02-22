# Freq Folly

> [!WARNING]
> This application is surprisingly intensive and not mega-optimised as its just a toy. May heat up your laptop etc, use with caution.

Little audio-toy that lets you bounce around a ball using the frequencies of your microphone. Try to get the ball into the highlighted circle to earn points.

## Technical Details

The audio analysis (`ffaa`) is written in rust and compiled to a WASM bundle. Due to some limitations when loading WASM from inside of an ["AudioWorklet"](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet)
I couldn't easily use wasm-pack or wasm-bindgen so I've just written the bindings manually w/ "extern C" style rust.

If anyone knows how to remedy this and make use of bindgen, I'm very curious haha.
