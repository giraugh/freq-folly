use std::{cell::RefCell, f32::consts::TAU, iter, sync::LazyLock};

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

/// Our FT is computed across this many samples
/// *effectively* controls what the time-range is for the FT
const SAMPLES_SIZE: usize = 256;

/// Setup an FFT planner (as we always ues the same size)
static FFT_PLANNER: LazyLock<Planner32> =
    LazyLock::new(|| Planner32::new(SAMPLES_SIZE, Direction::Forward));

/// Our "imaginary" component of each sample for FT is always 0
/// so just setup that up ahead of time
static FFT_IMAGS: [f32; SAMPLES_SIZE] = [0.0; SAMPLES_SIZE];

// Shared buffers
thread_local! {
    /// Buffer for javascript to send us samples
    pub static SAMPLE_IN_BUFF: RefCell<[f32; BLOCK_SIZE]> = const { RefCell::new([0.0; BLOCK_SIZE]) };

    /// A buffer for getting our FT output back out to javascript
    pub static FREQ_OUT_BUFF: RefCell<[f32; SAMPLES_SIZE / 2]> = const { RefCell::new([0.0; SAMPLES_SIZE / 2]) };

    /// Ring buffer for DSP
    pub static SAMPLES: RefCell<ConstGenericRingBuffer<f32, SAMPLES_SIZE>> = const { RefCell::new(ConstGenericRingBuffer::<f32, SAMPLES_SIZE>::new()) };
}

#[no_mangle]
pub extern "C" fn sample_in_ptr() -> *mut f32 {
    SAMPLE_IN_BUFF.with_borrow_mut(|b| b.as_mut_ptr())
}

#[no_mangle]
pub extern "C" fn freq_out_ptr() -> *mut f32 {
    FREQ_OUT_BUFF.with_borrow_mut(|b| b.as_mut_ptr())
}

#[no_mangle]
pub extern "C" fn freq_out_len() -> usize {
    FREQ_OUT_BUFF.with_borrow(|b| b.len())
}

// NOTE: we always assume that length is 128
#[no_mangle]
pub extern "C" fn process_samples() {
    // copy to ring buffer from array
    SAMPLE_IN_BUFF.with_borrow(|new_samples| {
        SAMPLES.with_borrow_mut(|rb| {
            rb.extend(new_samples.iter().cloned());
        })
    });

    // We ONLY start FT once we have enough points
    if SAMPLES.with_borrow(|b| b.len()) < SAMPLES_SIZE {
        return;
    }

    // Apply a window function to each real value
    let n = SAMPLES.with_borrow(|b| b.len()) as f32;
    let mut reals = SAMPLES.with_borrow(|b| {
        b.iter()
            .enumerate()
            .map(|(i, x)| hann(i, n) * *x)
            .collect::<Vec<_>>()
    });

    // Then do FT
    let mut imags = Vec::from(FFT_IMAGS);
    fft_32_with_opts_and_plan(&mut reals, &mut imags, &Options::default(), &FFT_PLANNER);

    // Now the data we want is the magnitude of all those vectors
    // so we can compute that and put them in an "out" buffer
    // NOTE: we only want the first half of the FT output (since in practice its mirrored for real-valued inputs)
    let mags = iter::zip(reals, imags)
        .map(|(a, b)| 2.0 * (a.powi(2) + b.powi(2)).sqrt())
        .take(SAMPLES_SIZE / 2);
    FREQ_OUT_BUFF.with_borrow_mut(|fout| {
        mags.enumerate().for_each(|(i, v)| {
            fout[i] = v;
        });
    })
}

fn hann(i: usize, n: f32) -> f32 {
    0.5 - 0.5 * f32::cos(TAU * (i as f32) / (n - 1.0))
}
