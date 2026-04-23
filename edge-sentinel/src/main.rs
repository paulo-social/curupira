use std::{
    collections::VecDeque,
    env,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::seq::SliceRandom;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use tokio::time::sleep;

const API_URL: &str = "http://localhost:8080/analyze";
const BUFFER_SECONDS: usize = 5;

#[derive(Debug, Deserialize)]
struct AnalyzeResponse {
    label: String,
    confidence: f32,
    persisted: bool,
}

#[derive(Clone)]
struct SharedBuffer {
    sample_rate: u32,
    samples: Arc<Mutex<VecDeque<f32>>>,
}

impl SharedBuffer {
    fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: Arc::new(Mutex::new(VecDeque::with_capacity(
                sample_rate as usize * BUFFER_SECONDS,
            ))),
        }
    }

    fn push(&self, sample: f32) {
        let mut guard = self.samples.lock().expect("audio buffer poisoned");
        let max_samples = self.sample_rate as usize * BUFFER_SECONDS;
        if guard.len() == max_samples {
            guard.pop_front();
        }
        guard.push_back(sample);
    }

    fn snapshot(&self) -> Vec<f32> {
        self.samples
            .lock()
            .expect("audio buffer poisoned")
            .iter()
            .copied()
            .collect()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    edge_log("Inicializando sentinela de borda.");
    if simulation_enabled() {
        edge_log("Modo de simulação habilitado.");
        run_simulation().await?;
    } else {
        edge_log("Modo microfone habilitado.");
        run_microphone_mode().await?;
    }
    Ok(())
}

fn simulation_enabled() -> bool {
    env::var("SIMULATION")
        .map(|value| is_simulation_value(&value))
        .unwrap_or(false)
}

fn is_simulation_value(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE" | "yes" | "YES")
}

async fn run_simulation() -> Result<()> {
    let samples_dir = env::var("SAMPLES_DIR").unwrap_or_else(|_| "samples".to_string());
    let paths = collect_wavs(Path::new(&samples_dir))?;
    let client = reqwest::Client::new();
    edge_log(format!(
        "Simulação pronta com {} arquivo(s) em `{samples_dir}`.",
        paths.len()
    ));

    loop {
        let path = paths
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| anyhow!("nenhum arquivo .wav encontrado em {}", samples_dir))?;

        let bytes = fs::read(path).with_context(|| format!("falha ao ler {}", path.display()))?;
        edge_log(format!(
            "Detectado som suspeito em `{}`: analisando...",
            path.file_name().unwrap().to_string_lossy()
        ));
        post_audio(&client, bytes, path.file_name().unwrap().to_string_lossy().to_string()).await?;
        sleep(Duration::from_secs(10)).await;
    }
}

async fn run_microphone_mode() -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("nenhum microfone padrão disponível"))?;
    let config = device.default_input_config()?;
    edge_log(format!(
        "Capturando áudio do dispositivo `{}` a {} Hz.",
        device.name().unwrap_or_else(|_| "desconhecido".to_string()),
        config.sample_rate().0
    ));
    let sample_rate = config.sample_rate().0;
    let buffer = SharedBuffer::new(sample_rate);
    let stream = build_stream(&device, &config.into(), buffer.clone())?;
    let client = reqwest::Client::new();

    stream.play()?;

    loop {
        sleep(Duration::from_secs(BUFFER_SECONDS as u64)).await;
        let snapshot = buffer.snapshot();
        if snapshot.len() < sample_rate as usize {
            continue;
        }

        let wav_bytes = samples_to_wav(snapshot, sample_rate)?;
        edge_log("Detectado som suspeito: analisando buffer capturado...");
        post_audio(&client, wav_bytes, "live-buffer.wav".to_string()).await?;
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: SharedBuffer,
) -> Result<cpal::Stream> {
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("erro de captura: {err}");

    let supported = device.default_input_config()?;
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let shared = buffer.clone();
            device.build_input_stream(
                config,
                move |data: &[f32], _| write_f32_input_data(data, channels, &shared),
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let shared = buffer.clone();
            device.build_input_stream(
                config,
                move |data: &[i16], _| write_i16_input_data(data, channels, &shared),
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let shared = buffer.clone();
            device.build_input_stream(
                config,
                move |data: &[u16], _| write_u16_input_data(data, channels, &shared),
                err_fn,
                None,
            )?
        }
        other => return Err(anyhow!("formato de áudio não suportado: {other:?}")),
    };

    Ok(stream)
}

fn write_f32_input_data(input: &[f32], channels: usize, buffer: &SharedBuffer) {
    for frame in input.chunks(channels) {
        let mono = frame.iter().sum::<f32>() / channels as f32;
        buffer.push(mono);
    }
}

fn write_i16_input_data(input: &[i16], channels: usize, buffer: &SharedBuffer) {
    for frame in input.chunks(channels) {
        let mono = frame
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .sum::<f32>()
            / channels as f32;
        buffer.push(mono);
    }
}

fn write_u16_input_data(input: &[u16], channels: usize, buffer: &SharedBuffer) {
    for frame in input.chunks(channels) {
        let mono = frame
            .iter()
            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
            .sum::<f32>()
            / channels as f32;
        buffer.push(mono);
    }
}

fn samples_to_wav(samples: Vec<f32>, sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
    for sample in samples {
        let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(scaled)?;
    }
    writer.finalize()?;

    Ok(cursor.into_inner())
}

fn collect_wavs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("falha ao listar {}", dir.display()))? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("wav"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    Ok(files)
}

async fn post_audio(client: &reqwest::Client, bytes: Vec<u8>, filename: String) -> Result<()> {
    let part = Part::bytes(bytes)
        .file_name(filename)
        .mime_str("audio/wav")?;
    let form = Form::new().part("file", part);

    let response = client.post(API_URL).multipart(form).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        edge_error(format!("Falha ao enviar áudio para a nuvem: backend respondeu {status}."));
        return Err(anyhow!("backend respondeu {status}: {body}"));
    }

    let result: AnalyzeResponse = response.json().await?;
    edge_log(format!(
        "Resposta da nuvem: {} detectada com {:.0}% de confiança.",
        format_event_label(&result.label),
        result.confidence
    ));

    if result.persisted {
        edge_log(format!(
            "Alerta crítico confirmado pela nuvem para {}.",
            format_event_label(&result.label)
        ));
    }

    Ok(())
}

fn format_event_label(label: &str) -> &'static str {
    match label {
        "motosserra" => "Motosserra",
        "tiro" => "Tiro",
        "chuva" => "Chuva",
        "ambiente" => "Som ambiente",
        _ => "Som desconhecido",
    }
}

fn edge_log(message: impl AsRef<str>) {
    println!("[Curupira-Edge] {}", message.as_ref());
}

fn edge_error(message: impl AsRef<str>) {
    eprintln!("[Curupira-Edge] {}", message.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_buffer_keeps_only_recent_samples() {
        let buffer = SharedBuffer::new(2);

        for sample in 0..12 {
            buffer.push(sample as f32);
        }

        assert_eq!(buffer.snapshot(), vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]);
    }

    #[test]
    fn simulation_value_parser_accepts_expected_truthy_inputs() {
        assert!(is_simulation_value("1"));
        assert!(is_simulation_value("true"));
        assert!(is_simulation_value("TRUE"));
        assert!(is_simulation_value("yes"));
        assert!(is_simulation_value("YES"));
        assert!(!is_simulation_value("false"));
        assert!(!is_simulation_value("0"));
    }

    #[test]
    fn samples_to_wav_roundtrip_preserves_sample_rate_and_length() {
        let wav = samples_to_wav(vec![-1.0, 0.0, 1.0], 22_050).expect("wav should be generated");
        let reader = hound::WavReader::new(Cursor::new(wav)).expect("wav should be readable");

        assert_eq!(reader.spec().sample_rate, 22_050);
        assert_eq!(reader.duration(), 3);
    }

    #[test]
    fn collect_wavs_filters_non_wav_files() {
        let unique = format!(
            "curupira-edge-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).expect("temp directory should be created");
        fs::write(dir.join("sample.wav"), b"wav").expect("wav file should be written");
        fs::write(dir.join("notes.txt"), b"text").expect("text file should be written");

        let wavs = collect_wavs(&dir).expect("wavs should be collected");

        assert_eq!(wavs.len(), 1);
        assert_eq!(wavs[0].file_name().and_then(|name| name.to_str()), Some("sample.wav"));

        fs::remove_dir_all(&dir).expect("temp directory should be removed");
    }

    #[test]
    fn format_event_label_maps_expected_names() {
        assert_eq!(format_event_label("tiro"), "Tiro");
        assert_eq!(format_event_label("desconhecido"), "Som desconhecido");
    }
}
