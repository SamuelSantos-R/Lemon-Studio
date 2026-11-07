// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Sample Killer's analysis: one FFT frame every half window gives a chromagram, the energy per pitch class taken from
//! the spectrum's peaks, so chords and full loops count. All buffers are sized in `configure`; `push` never allocates.

use std::f32::consts::PI;

/// Frames quieter than this (RMS, about -55 dBFS) add nothing.
const SILENCE_RMS: f32 = 0.0018;
const CHROMA_LO_HZ: f32 = 60.0;
const CHROMA_HI_HZ: f32 = 5000.0;

pub struct Analyzer {
    n: usize,
    hop: usize,
    ring: Vec<f32>,
    write: usize,
    filled: usize,
    since: usize,
    window: Vec<f32>,
    tw_re: Vec<f32>,
    tw_im: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    mag: Vec<f32>,
    sr: f32,
    /// Chroma of the last frame, summing to 1 (all zero when the frame was silent).
    pub frame_chroma: [f32; 12],
}

impl Analyzer {
    pub fn new() -> Self {
        let mut a = Analyzer {
            n: 0,
            hop: 0,
            ring: Vec::new(),
            write: 0,
            filled: 0,
            since: 0,
            window: Vec::new(),
            tw_re: Vec::new(),
            tw_im: Vec::new(),
            re: Vec::new(),
            im: Vec::new(),
            mag: Vec::new(),
            sr: 44100.0,
            frame_chroma: [0.0; 12],
        };
        a.configure(44100.0);
        a
    }

    /// Sizes the buffers for `sr`: about 185 ms windows, so semitones stay apart down to roughly 90 Hz.
    pub fn configure(&mut self, sr: f32) {
        let n = if sr <= 50_000.0 { 8192 } else if sr <= 100_000.0 { 16384 } else { 32768 };
        self.sr = sr;
        self.n = n;
        self.hop = n / 2;
        self.ring = vec![0.0; n];
        self.window = (0..n).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()).collect();
        self.tw_re = (0..n / 2).map(|k| (-2.0 * PI * k as f32 / n as f32).cos()).collect();
        self.tw_im = (0..n / 2).map(|k| (-2.0 * PI * k as f32 / n as f32).sin()).collect();
        self.re = vec![0.0; n];
        self.im = vec![0.0; n];
        self.mag = vec![0.0; n / 2];
        self.reset();
    }

    /// Samples between frames.
    pub fn hop(&self) -> usize {
        self.hop
    }

    pub fn reset(&mut self) {
        self.ring.iter_mut().for_each(|s| *s = 0.0);
        self.write = 0;
        self.filled = 0;
        self.since = 0;
        self.frame_chroma = [0.0; 12];
    }

    /// Adds one mono sample; true when a new frame was analyzed (read `frame_chroma`).
    pub fn push(&mut self, x: f32) -> bool {
        self.ring[self.write] = x;
        self.write = (self.write + 1) & (self.n - 1);
        self.filled = (self.filled + 1).min(self.n);
        self.since += 1;
        if self.since >= self.hop && self.filled == self.n {
            self.since = 0;
            self.frame();
            true
        } else {
            false
        }
    }

    fn frame(&mut self) {
        let n = self.n;
        let mut energy = 0.0f32;
        for i in 0..n {
            let s = self.ring[(self.write + i) & (n - 1)];
            energy += s * s;
            self.re[i] = s * self.window[i];
            self.im[i] = 0.0;
        }
        if (energy / n as f32).sqrt() < SILENCE_RMS {
            self.frame_chroma = [0.0; 12];
            return;
        }
        fft(&mut self.re, &mut self.im, &self.tw_re, &self.tw_im);
        for k in 0..n / 2 {
            self.mag[k] = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt();
        }
        self.chroma();
    }

    /// Energy per pitch class from the spectrum's peaks: each peak's frequency is refined between bins and credited to
    /// its nearest semitone, less the further it is from that semitone's centre.
    fn chroma(&mut self) {
        let bin_hz = self.sr / self.n as f32;
        let lo = ((CHROMA_LO_HZ / bin_hz) as usize).max(2);
        let hi = ((CHROMA_HI_HZ / bin_hz) as usize).min(self.n / 2 - 2);
        let max = self.mag[lo..hi].iter().copied().fold(0.0f32, f32::max);
        let floor = max * 0.01;
        let mut c = [0.0f32; 12];
        for k in lo..hi {
            let m = self.mag[k];
            if m < floor || m <= self.mag[k - 1] || m < self.mag[k + 1] {
                continue;
            }
            let (a, b, g) = (self.mag[k - 1].max(1e-12).ln(), m.max(1e-12).ln(), self.mag[k + 1].max(1e-12).ln());
            let den = a - 2.0 * b + g;
            let delta = if den.abs() > 1e-9 { (0.5 * (a - g) / den).clamp(-0.5, 0.5) } else { 0.0 };
            let midi = 69.0 + 12.0 * (((k as f32 + delta) * bin_hz) / 440.0).log2();
            let nearest = midi.round();
            let off = (midi - nearest).abs();
            if off > 0.4 {
                continue;
            }
            let pc = (nearest as i32).rem_euclid(12) as usize;
            c[pc] += m.sqrt() * (1.0 - off * 2.0);
        }
        let sum: f32 = c.iter().sum();
        self.frame_chroma = if sum > 0.0 { c.map(|v| v / sum) } else { [0.0; 12] };
    }
}

/// In-place iterative radix-2 FFT; `re.len()` is a power of two and the twiddles are e^(-2πik/n) for k < n/2.
fn fft(re: &mut [f32], im: &mut [f32], tw_re: &[f32], tw_im: &[f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let step = n / len;
        for start in (0..n).step_by(len) {
            for k in 0..half {
                let (wr, wi) = (tw_re[k * step], tw_im[k * step]);
                let (a, b) = (start + k, start + k + half);
                let xr = re[b] * wr - im[b] * wi;
                let xi = re[b] * wi + im[b] * wr;
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(sr: f32, secs: f32, f: impl Fn(f32) -> f32) -> Analyzer {
        let mut a = Analyzer::new();
        a.configure(sr);
        for i in 0..(sr * secs) as usize {
            a.push(f(i as f32 / sr));
        }
        a
    }

    #[test]
    fn chroma_of_a_chord_names_its_notes() {
        // A minor triad: A3, C4, E4.
        let a = run(44100.0, 1.0, |t| [220.0f32, 261.63, 329.63].iter().map(|f| (2.0 * PI * f * t).sin()).sum::<f32>() * 0.2);
        let mut top: Vec<usize> = (0..12).collect();
        top.sort_by(|&x, &y| a.frame_chroma[y].partial_cmp(&a.frame_chroma[x]).unwrap());
        let mut three = top[..3].to_vec();
        three.sort();
        assert_eq!(three, vec![0, 4, 9]);
    }

    #[test]
    fn silence_reads_nothing() {
        let a = run(44100.0, 1.0, |_| 0.0);
        assert_eq!(a.frame_chroma, [0.0; 12]);
    }
}
