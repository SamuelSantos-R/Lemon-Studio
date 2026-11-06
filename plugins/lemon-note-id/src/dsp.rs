// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! YIN pitch detection for Lemon Note ID. Detects the fundamental frequency of a monophonic signal
//! down to ~20 Hz (sub-bass 808 range) with a 4096-sample ring buffer.

const YIN_SIZE: usize = 4096;
const YIN_HALF: usize = YIN_SIZE / 2;
const YIN_THRESHOLD: f32 = 0.12;

pub struct YinDetector {
    ring: Box<[f32; YIN_SIZE]>,
    write: usize,
    filled: bool,
    diff: Box<[f32; YIN_HALF]>,
    cmndf: Box<[f32; YIN_HALF]>,
}

impl YinDetector {
    pub fn new() -> Self {
        YinDetector {
            ring: Box::new([0.0; YIN_SIZE]),
            write: 0,
            filled: false,
            diff: Box::new([0.0; YIN_HALF]),
            cmndf: Box::new([0.0; YIN_HALF]),
        }
    }

    pub fn push(&mut self, s: f32) {
        self.ring[self.write] = s;
        self.write += 1;
        if self.write >= YIN_SIZE {
            self.write = 0;
            self.filled = true;
        }
    }

    /// Run YIN detection. Returns the fundamental frequency in Hz, or None if no confident pitch found.
    /// Only call every N blocks (not every sample) to limit CPU use.
    pub fn detect(&mut self, sample_rate: f32) -> Option<f32> {
        if !self.filled || sample_rate < 1.0 {
            return None;
        }

        let w = YIN_HALF;
        let start = self.write; // oldest sample in the ring (since we just wrapped)

        // Step 1: difference function d[τ] = Σ (x[j] - x[j+τ])²
        self.diff[0] = 0.0;
        for tau in 1..w {
            let mut sum = 0.0f32;
            for j in 0..w {
                let a = self.ring[(start + j) % YIN_SIZE];
                let b = self.ring[(start + j + tau) % YIN_SIZE];
                let d = a - b;
                sum += d * d;
            }
            self.diff[tau] = sum;
        }

        // Step 2: cumulative mean normalized difference function
        self.cmndf[0] = 1.0;
        let mut running = 0.0f32;
        for tau in 1..w {
            running += self.diff[tau];
            self.cmndf[tau] = if running > 0.0 {
                self.diff[tau] * tau as f32 / running
            } else {
                1.0
            };
        }

        // Step 3: absolute threshold — find first τ where CMNDF < threshold
        let tau_min = ((sample_rate / 2000.0) as usize).max(1);
        let tau_max = ((sample_rate / 20.0) as usize).min(w.saturating_sub(2));

        let mut best_tau = None;
        let mut tau = tau_min;
        while tau < tau_max {
            if self.cmndf[tau] < YIN_THRESHOLD {
                while tau + 1 < tau_max && self.cmndf[tau + 1] < self.cmndf[tau] {
                    tau += 1;
                }
                best_tau = Some(tau);
                break;
            }
            tau += 1;
        }

        // Fall back to global minimum if threshold not met, with stricter confidence check
        let tau = best_tau.or_else(|| {
            (tau_min..tau_max).min_by(|&a, &b| self.cmndf[a].partial_cmp(&self.cmndf[b]).unwrap_or(std::cmp::Ordering::Equal))
        })?;

        if self.cmndf[tau] > 0.25 {
            return None;
        }

        // Step 4: parabolic interpolation around the minimum
        let tau_f = if tau > 0 && tau + 1 < w {
            let a = self.cmndf[tau - 1];
            let b = self.cmndf[tau];
            let c = self.cmndf[tau + 1];
            let denom = a - 2.0 * b + c;
            if denom.abs() > 1e-10 {
                tau as f32 + 0.5 * (a - c) / denom
            } else {
                tau as f32
            }
        } else {
            tau as f32
        };

        let freq = sample_rate / tau_f;
        if freq >= 20.0 && freq <= 4200.0 { Some(freq) } else { None }
    }
}
