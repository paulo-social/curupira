from __future__ import annotations

import argparse
import math
import random
import wave
from pathlib import Path


SAMPLE_RATE = 16_000
SECONDS = 2.048
TOTAL_SAMPLES = int(SAMPLE_RATE * SECONDS)
CLASSES = ["ambiente", "chuva", "motosserra", "tiro"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Gera dataset sintetico para teste.")
    parser.add_argument("--output-dir", type=Path, default=Path("training/data"))
    parser.add_argument("--samples-per-class", type=int, default=24)
    parser.add_argument("--seed", type=int, default=42)
    return parser.parse_args()


def clamp_sample(value: float) -> int:
    value = max(-1.0, min(1.0, value))
    return int(value * 32767)


def write_wav(path: Path, samples: list[float]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)
        wav_file.setframerate(SAMPLE_RATE)
        frames = bytearray()
        for sample in samples:
            frames += clamp_sample(sample).to_bytes(2, byteorder="little", signed=True)
        wav_file.writeframes(frames)


def envelope(index: int, attack: int, decay: int) -> float:
    if index < attack:
        return index / max(1, attack)
    if index > TOTAL_SAMPLES - decay:
        return max(0.0, (TOTAL_SAMPLES - index) / max(1, decay))
    return 1.0


def ambiente_sample(rng: random.Random) -> list[float]:
    samples: list[float] = []
    hum_freq = rng.uniform(80.0, 180.0)
    breeze_freq = rng.uniform(300.0, 900.0)
    for i in range(TOTAL_SAMPLES):
        t = i / SAMPLE_RATE
        hum = 0.06 * math.sin(2 * math.pi * hum_freq * t)
        breeze = 0.025 * math.sin(2 * math.pi * breeze_freq * t)
        noise = rng.uniform(-0.035, 0.035)
        samples.append(hum + breeze + noise)
    return samples


def chuva_sample(rng: random.Random) -> list[float]:
    samples: list[float] = []
    for i in range(TOTAL_SAMPLES):
        t = i / SAMPLE_RATE
        base = 0.03 * math.sin(2 * math.pi * rng.uniform(1800.0, 2600.0) * t)
        droplet = rng.uniform(-0.12, 0.12) if rng.random() < 0.06 else 0.0
        noise = rng.uniform(-0.08, 0.08)
        samples.append(base + droplet + noise)
    return samples


def motosserra_sample(rng: random.Random) -> list[float]:
    samples: list[float] = []
    base_freq = rng.uniform(95.0, 135.0)
    mod_freq = rng.uniform(8.0, 16.0)
    for i in range(TOTAL_SAMPLES):
        t = i / SAMPLE_RATE
        mod = 1.0 + 0.35 * math.sin(2 * math.pi * mod_freq * t)
        engine = 0.22 * mod * math.sin(2 * math.pi * base_freq * t)
        harmonic = 0.14 * math.sin(2 * math.pi * base_freq * 2.2 * t)
        rasp = rng.uniform(-0.12, 0.12)
        samples.append(engine + harmonic + rasp)
    return samples


def tiro_sample(rng: random.Random) -> list[float]:
    samples = [rng.uniform(-0.01, 0.01) for _ in range(TOTAL_SAMPLES)]
    shot_count = rng.randint(1, 3)
    for _ in range(shot_count):
        center = rng.randint(1500, TOTAL_SAMPLES - 2500)
        width = rng.randint(200, 600)
        ring_freq = rng.uniform(700.0, 1800.0)
        for offset in range(width):
            idx = center + offset
            if idx >= TOTAL_SAMPLES:
                break
            t = offset / SAMPLE_RATE
            decay = math.exp(-10 * t)
            blast = 0.95 * decay * math.sin(2 * math.pi * ring_freq * t)
            crack = rng.uniform(-1.0, 1.0) * decay * 0.8
            samples[idx] += blast + crack
    return samples


def generate_class(class_name: str, count: int, output_dir: Path, rng: random.Random) -> None:
    generators = {
        "ambiente": ambiente_sample,
        "chuva": chuva_sample,
        "motosserra": motosserra_sample,
        "tiro": tiro_sample,
    }
    generator = generators[class_name]

    for index in range(count):
        sample_rng = random.Random(rng.randint(0, 10_000_000))
        samples = generator(sample_rng)
        path = output_dir / class_name / f"{class_name}_{index:03d}.wav"
        write_wav(path, samples)


def main() -> None:
    args = parse_args()
    rng = random.Random(args.seed)

    for class_name in CLASSES:
        generate_class(class_name, args.samples_per_class, args.output_dir, rng)

    print(f"dataset sintetico gerado em {args.output_dir}")
    print(f"classes: {', '.join(CLASSES)}")
    print(f"amostras por classe: {args.samples_per_class}")


if __name__ == "__main__":
    main()

