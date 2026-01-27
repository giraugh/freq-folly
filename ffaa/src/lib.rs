use std::{f32::consts::TAU, iter, sync::LazyLock};

use phastft::{
    fft_32_with_opts_and_plan,
    options::Options,
    planner::{Direction, Planner32},
};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

mod utils;

/// How many samples we accept from javascript at one time
/// this is locked at 128 as per the current browser audio APIs
const BLOCK_SIZE: usize = 128;

/// Buffer for javascript to send us samples
static mut SAMPLE_IN_BUFF: [f32; BLOCK_SIZE] = [0.0; BLOCK_SIZE];

/// Our FT is computed across this many samples
/// *effectively* controls what the time-range is for the FT
const SAMPLES_SIZE: usize = 256;

/// Ring buffer for DSP
static mut SAMPLES: ConstGenericRingBuffer<f32, SAMPLES_SIZE> =
    ConstGenericRingBuffer::<f32, SAMPLES_SIZE>::new();

/// A buffer for getting our FT output back out to javascript
static mut FREQ_OUT_BUFF: [f32; SAMPLES_SIZE / 2] = [0.0; SAMPLES_SIZE / 2];

/// Setup an FFT planner (as we always ues the same size)
static FFT_PLANNER: LazyLock<Planner32> =
    LazyLock::new(|| Planner32::new(SAMPLES_SIZE, Direction::Forward));

/// Our "imaginary" component of each sample for FT is always 0
/// so just setup that up ahead of time
static FFT_IMAGS: [f32; SAMPLES_SIZE] = [0.0; SAMPLES_SIZE];

#[no_mangle]
pub extern "C" fn sample_in_ptr() -> *mut f32 {
    unsafe { SAMPLE_IN_BUFF.as_mut_ptr() }
}

#[no_mangle]
pub extern "C" fn freq_out_ptr() -> *mut f32 {
    unsafe { FREQ_OUT_BUFF.as_mut_ptr() }
}

#[no_mangle]
pub extern "C" fn freq_out_len() -> usize {
    unsafe { FREQ_OUT_BUFF.len() }
}

// wip
struct SpectrumConfig {
    /// How many bins in output spectrum?
    /// NOTE: is logarithmic
    spectrum_size: usize,

    /// Minimum frequency in Hz
    min_freq: f32,

    /// Maximum frequency in Hz
    max_freq: f32,
}

// NOTE: we always assume that length is 128
#[no_mangle]
pub extern "C" fn process_samples() {
    // copy to ring buffer from array
    let new_samples = unsafe { &SAMPLE_IN_BUFF[..] };
    unsafe {
        SAMPLES.extend(new_samples.iter().cloned());
    }

    // We ONLY start FT once we have enough points
    if unsafe { SAMPLES.len() } < SAMPLES_SIZE {
        return;
    }

    // Then do FT
    // for now just return the length of the ring
    // TODO: apply a window function to the reals here (e.g hann)
    let n = unsafe { SAMPLES.len() as f32 };
    let mut reals = unsafe {
        SAMPLES
            .iter()
            .enumerate()
            .map(|(i, x)| hann(i, n) * *x)
            .collect::<Vec<_>>()
    };
    let mut imags = Vec::from(FFT_IMAGS);
    fft_32_with_opts_and_plan(&mut reals, &mut imags, &Options::default(), &FFT_PLANNER);

    // Now the data we want is the magnitude of all those vectors
    // so we can compute that and put them in an "out" buffer
    // NOTE: we only want the first half of the FT output (since in practice its mirrored for real-valued inputs)
    let mags = iter::zip(reals, imags)
        .map(|(a, b)| 2.0 * (a.powi(2) + b.powi(2)).sqrt())
        .take(SAMPLES_SIZE / 2);
    mags.enumerate().for_each(|(i, v)| unsafe {
        FREQ_OUT_BUFF[i] = v;
    });
}

fn hann(i: usize, n: f32) -> f32 {
    0.5 - 0.5 * f32::cos(TAU * (i as f32) / (n - 1.0))
}
