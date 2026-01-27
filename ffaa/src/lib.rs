use std::{
    cell::{Cell, RefCell},
    f32::consts::TAU,
    iter,
    sync::LazyLock,
};

use itertools::Itertools;
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
const SAMPLES_SIZE: usize = 4096;

/// How many bands we want in our output spectrum
const BANDS_COUNT: usize = 70;

/// Setup an FFT planner (as we always ues the same size)
static FFT_PLANNER: LazyLock<Planner32> =
    LazyLock::new(|| Planner32::new(SAMPLES_SIZE, Direction::Forward));

/// Our "imaginary" component of each sample for FT is always 0
/// so just setup that up ahead of time
static FFT_IMAGS: [f32; SAMPLES_SIZE] = [0.0; SAMPLES_SIZE];

// Mutable buffers and values
// some of which are shared...
thread_local! {
    /// Audio sample rate
    pub static SAMPLE_RATE: Cell<usize> = const { Cell::new(48_000) };

    /// Buffer for javascript to send us samples
    pub static SAMPLE_IN_BUFF: RefCell<[f32; BLOCK_SIZE]> = const { RefCell::new([0.0; BLOCK_SIZE]) };

    /// A buffer for getting our FT output back out to javascript
    pub static FREQ_OUT_BUFF: RefCell<[f32; BANDS_COUNT]> = const { RefCell::new([0.0; BANDS_COUNT]) };

    /// Ring buffer for DSP
    pub static SAMPLES: RefCell<ConstGenericRingBuffer<f32, SAMPLES_SIZE>> = const { RefCell::new(ConstGenericRingBuffer::<f32, SAMPLES_SIZE>::new()) };
}

#[no_mangle]
pub extern "C" fn set_sample_rate(rate_hz: usize) {
    SAMPLE_RATE.set(rate_hz);
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
    let magnitudes = iter::zip(reals, imags)
        .map(|(a, b)| 2.0 * (a.powi(2) + b.powi(2)).sqrt())
        .collect_array::<SAMPLES_SIZE>()
        .unwrap();

    // Calculate frequency bands from the sample rate n stuff
    // Then calculate the DB for each band and assign to output
    let spectrum_bands = get_spectrum_bands();
    FREQ_OUT_BUFF.with_borrow_mut(|fout| {
        spectrum_bands
            .into_iter()
            .enumerate()
            .for_each(|(i, (start_bin, end_bin))| {
                let sum = &magnitudes[start_bin..end_bin].iter().sum();
                let energy = sum / (end_bin - start_bin) as f32;
                let out = energy.powf(0.35); // scale it down a tad to reduce spikiness :relieved:
                fout[i] = out;
            });
    });
}

fn get_spectrum_bands() -> Vec<(usize, usize)> {
    // Calculate frequency bands from the sample rate n stuff
    let min_freq = 70.0; //20.0;
    let sample_rate = SAMPLE_RATE.get() as f32;
    let max_freq = 1200.0_f32; // sample_rate / 2.0;
    let spectrum_bins = (0..=BANDS_COUNT).map(|i| {
        let freq = min_freq * (max_freq / min_freq).powf((i as f32) / (BANDS_COUNT as f32));
        let bin = freq * (SAMPLES_SIZE as f32) / sample_rate;
        bin.floor() as usize
    });

    let spectrum_bands = spectrum_bins.tuple_windows().map(|(start_bin, end_bin)| {
        let start_bin = usize::max(start_bin, 1);
        let end_bin = usize::max(end_bin, start_bin + 1);
        (start_bin, end_bin)
    });

    spectrum_bands.collect()
}

#[test]
fn test_bands() {
    let bands = get_spectrum_bands();
    dbg!(bands);
    panic!()
}

fn hann(i: usize, n: f32) -> f32 {
    0.5 - 0.5 * f32::cos(TAU * (i as f32) / (n - 1.0))
}
