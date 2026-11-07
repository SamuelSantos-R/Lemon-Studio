//! `cargo run -p sample-killer --example analyze -- file.wav`: what Sample Killer hears in a mono float WAV
//! (`afconvert -f WAVE -d LEF32@44100 -c 1 in.mp3 out.wav`), second by second.

use sample_killer::{dsp::Analyzer, theory};

fn main() {
    let path = std::env::args().nth(1).expect("wav path");
    let bytes = std::fs::read(&path).expect("read");
    let sr = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as f32;
    let mut i = 12;
    let data = loop {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes(bytes[i + 4..i + 8].try_into().unwrap()) as usize;
        if id == b"data" {
            break &bytes[i + 8..i + 8 + len];
        }
        i += 8 + len + (len & 1);
    };
    let samples: Vec<f32> = data.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
    let mut a = Analyzer::new();
    a.configure(sr);
    let decay = (-(a.hop() as f32) / (sr * 60.0)).exp();
    let mut acc = [0.0f32; 12];
    let mut total = [0.0f32; 12];
    for (n, &x) in samples.iter().enumerate() {
        if a.push(x) {
            for k in 0..12 {
                acc[k] = acc[k] * decay + a.frame_chroma[k];
                total[k] += a.frame_chroma[k];
            }
        }
        if n % sr as usize == 0 && n > 0 {
            let k = theory::estimate_key(&acc);
            println!("{:>3}s  {:?}", n / sr as usize, k.map(|k| format!("{}{} {:.0}%", theory::NOTE_NAMES[k.root as usize], if k.minor { "m" } else { "" }, k.confidence * 100.0)));
        }
    }
    let max = total.iter().copied().fold(0.0, f32::max);
    for k in 0..12 {
        println!("{:<3}{:5.2} {}", theory::NOTE_NAMES[k], total[k] / max, "#".repeat((total[k] / max * 40.0) as usize));
    }
    println!("whole file: {:?}", theory::estimate_key(&total));
}
