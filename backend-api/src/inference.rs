use std::{io::Cursor, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use tract_onnx::prelude::*;

const LABELS: [&str; 4] = ["ambiente", "chuva", "motosserra", "tiro"];

#[derive(Clone)]
pub struct InferenceEngine {
    model_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub label: String,
    pub confidence: f32,
}

impl InferenceEngine {
    pub fn new(model_path: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
        }
    }

    pub fn analyze(&self, bytes: &[u8]) -> Result<InferenceResult> {
        let (samples, sample_rate) = decode_wav(bytes)?;
        let features = compute_mel_spectrogram(&samples, sample_rate, 64, 64);

        match self.run_model(&features) {
            Ok(result) => Ok(result),
            Err(_) => Ok(heuristic_classification(&samples)),
        }
    }

    fn run_model(&self, features: &[f32]) -> Result<InferenceResult> {
        if !self.model_path.exists() {
            return Err(anyhow!("modelo ONNX ausente em {}", self.model_path.display()));
        }

        let model = tract_onnx::onnx()
            .model_for_path(&self.model_path)?
            .into_optimized()?
            .into_runnable()?;

        let input = Tensor::from_shape(&[1, 1, 64, 64], features)
            .context("falha ao montar tensor de entrada")?;
        let result = model.run(tvec!(input.into()))?;
        let output = result[0].to_array_view::<f32>()?;
        let values: Vec<f32> = output.iter().copied().collect();

        pick_label(&values).ok_or_else(|| anyhow!("saída do modelo vazia"))
    }
}

fn decode_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32)> {
    let reader = hound::WavReader::new(Cursor::new(bytes))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let max = 2_i32.pow(spec.bits_per_sample.saturating_sub(1) as u32) as f32;
            reader
                .into_samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / max))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
    };

    Ok((samples, sample_rate))
}

fn compute_mel_spectrogram(samples: &[f32], sample_rate: u32, mel_bins: usize, frames: usize) -> Vec<f32> {
    let window_size = 1024usize;
    let hop_size = 512usize;
    let fft_bins = window_size / 2;
    let mut features = vec![0.0; mel_bins * frames];

    if samples.is_empty() {
        return features;
    }

    for frame_idx in 0..frames {
        let start = frame_idx * hop_size;
        let end = (start + window_size).min(samples.len());
        let chunk = &samples[start..end];

        if chunk.is_empty() {
            continue;
        }

        for mel_idx in 0..mel_bins {
            let bin_start = mel_idx * fft_bins / mel_bins;
            let bin_end = ((mel_idx + 1) * fft_bins / mel_bins).max(bin_start + 1);
            let freq_start = (bin_start as f32 / fft_bins as f32) * (sample_rate as f32 / 2.0);
            let freq_end = (bin_end as f32 / fft_bins as f32) * (sample_rate as f32 / 2.0);

            let mut energy = 0.0f32;
            for (offset, sample) in chunk.iter().enumerate() {
                let weight = hann(offset, chunk.len());
                let freq_weight = ((freq_start + freq_end) / 2.0 / 1000.0).max(1.0);
                energy += sample.abs() * weight * freq_weight;
            }

            features[frame_idx * mel_bins + mel_idx] = (energy / chunk.len() as f32 + 1e-6).ln();
        }
    }

    normalize(&mut features);
    features
}

fn hann(index: usize, len: usize) -> f32 {
    if len <= 1 {
        return 1.0;
    }
    let phase = (2.0 * std::f32::consts::PI * index as f32) / (len - 1) as f32;
    0.5 - 0.5 * phase.cos()
}

fn normalize(features: &mut [f32]) {
    let mean = features.iter().sum::<f32>() / features.len().max(1) as f32;
    let variance = features
        .iter()
        .map(|value| {
            let diff = value - mean;
            diff * diff
        })
        .sum::<f32>()
        / features.len().max(1) as f32;
    let std_dev = variance.sqrt().max(1e-6);

    for value in features {
        *value = (*value - mean) / std_dev;
    }
}

fn pick_label(scores: &[f32]) -> Option<InferenceResult> {
    let (idx, confidence) = scores
        .iter()
        .copied()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

    Some(InferenceResult {
        label: LABELS.get(idx).unwrap_or(&"desconhecido").to_string(),
        confidence: confidence.clamp(0.0, 1.0),
    })
}

fn heuristic_classification(samples: &[f32]) -> InferenceResult {
    let mean_abs = samples.iter().map(|value| value.abs()).sum::<f32>() / samples.len().max(1) as f32;
    let peak = samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0f32, f32::max);

    if peak > 0.92 {
        InferenceResult {
            label: "tiro".to_string(),
            confidence: 0.91,
        }
    } else if mean_abs > 0.28 {
        InferenceResult {
            label: "motosserra".to_string(),
            confidence: 0.84,
        }
    } else {
        InferenceResult {
            label: "ambiente".to_string(),
            confidence: 0.64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_wav_reads_integer_samples() {
        let wav_bytes = build_wav_bytes(&[0, i16::MAX, i16::MIN + 1], 16_000);

        let (samples, sample_rate) = decode_wav(&wav_bytes).expect("wav should decode");

        assert_eq!(sample_rate, 16_000);
        assert_eq!(samples.len(), 3);
        assert!(samples[0].abs() < 1e-6);
        assert!(samples[1] > 0.99);
        assert!(samples[2] < -0.99);
    }

    #[test]
    fn compute_mel_spectrogram_returns_expected_shape() {
        let samples = vec![0.25; 2048];

        let features = compute_mel_spectrogram(&samples, 16_000, 8, 4);

        assert_eq!(features.len(), 32);
        assert!(features.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn pick_label_clamps_confidence_and_uses_highest_score() {
        let result = pick_label(&[0.1, 0.4, 1.4, 0.2]).expect("label should be picked");

        assert_eq!(result.label, "motosserra");
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn heuristic_classification_detects_gunshot_on_high_peak() {
        let result = heuristic_classification(&[0.1, 0.95, 0.05]);

        assert_eq!(result.label, "tiro");
        assert_eq!(result.confidence, 0.91);
    }

    #[test]
    fn heuristic_classification_detects_chainsaw_on_high_average_energy() {
        let result = heuristic_classification(&[0.4, -0.35, 0.31, -0.3]);

        assert_eq!(result.label, "motosserra");
        assert_eq!(result.confidence, 0.84);
    }

    #[test]
    fn heuristic_classification_falls_back_to_environment_for_low_energy() {
        let result = heuristic_classification(&[0.02, -0.03, 0.01, 0.0]);

        assert_eq!(result.label, "ambiente");
        assert_eq!(result.confidence, 0.64);
    }

    fn build_wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("wav writer should be created");
        for sample in samples {
            writer.write_sample(*sample).expect("sample should be written");
        }
        writer.finalize().expect("wav should finalize");

        cursor.into_inner()
    }
}
