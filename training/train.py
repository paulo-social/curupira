from __future__ import annotations

import argparse
import json
import math
import random
from dataclasses import dataclass
from pathlib import Path

import torch
import torchaudio
from torch import nn
from torch.utils.data import DataLoader, Dataset, random_split


CLASSES = ["ambiente", "chuva", "motosserra", "tiro"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Treina o modelo de audio do Curupira.")
    parser.add_argument("--data-dir", type=Path, default=Path("training/data"))
    parser.add_argument("--output-dir", type=Path, default=Path("training/runs"))
    parser.add_argument("--sample-rate", type=int, default=16_000)
    parser.add_argument("--duration", type=float, default=2.048)
    parser.add_argument("--n-mels", type=int, default=64)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--learning-rate", type=float, default=1e-3)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    return parser.parse_args()


def seed_everything(seed: int) -> None:
    random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)


@dataclass
class AudioExample:
    path: Path
    class_index: int


class AudioDataset(Dataset):
    def __init__(self, data_dir: Path, sample_rate: int, duration: float, n_mels: int) -> None:
        self.data_dir = data_dir
        self.sample_rate = sample_rate
        self.target_samples = int(sample_rate * duration)
        self.n_mels = n_mels
        self.examples = self._scan()
        self.resampler_cache: dict[int, torchaudio.transforms.Resample] = {}
        self.mel_spec = torchaudio.transforms.MelSpectrogram(
            sample_rate=sample_rate,
            n_fft=1024,
            hop_length=512,
            n_mels=n_mels,
            power=2.0,
        )
        self.to_db = torchaudio.transforms.AmplitudeToDB(stype="power")

    def _scan(self) -> list[AudioExample]:
        examples: list[AudioExample] = []
        for class_index, class_name in enumerate(CLASSES):
            class_dir = self.data_dir / class_name
            if not class_dir.exists():
                continue
            for path in sorted(class_dir.rglob("*.wav")):
                examples.append(AudioExample(path=path, class_index=class_index))
        if not examples:
            raise RuntimeError(
                "Nenhum arquivo .wav encontrado. Use training/data/<classe>/*.wav."
            )
        return examples

    def __len__(self) -> int:
        return len(self.examples)

    def __getitem__(self, index: int) -> tuple[torch.Tensor, torch.Tensor]:
        example = self.examples[index]
        waveform, sample_rate = torchaudio.load(example.path)
        waveform = waveform.mean(dim=0, keepdim=True)

        if sample_rate != self.sample_rate:
            waveform = self._resample(waveform, sample_rate)

        waveform = self._fix_length(waveform)
        features = self.mel_spec(waveform)
        features = self.to_db(features)
        features = self._fix_time_axis(features)
        features = self._normalize(features)
        return features, torch.tensor(example.class_index, dtype=torch.long)

    def _resample(self, waveform: torch.Tensor, original_rate: int) -> torch.Tensor:
        transform = self.resampler_cache.get(original_rate)
        if transform is None:
            transform = torchaudio.transforms.Resample(
                orig_freq=original_rate,
                new_freq=self.sample_rate,
            )
            self.resampler_cache[original_rate] = transform
        return transform(waveform)

    def _fix_length(self, waveform: torch.Tensor) -> torch.Tensor:
        current = waveform.shape[-1]
        if current > self.target_samples:
            return waveform[..., : self.target_samples]
        if current < self.target_samples:
            padding = self.target_samples - current
            return nn.functional.pad(waveform, (0, padding))
        return waveform

    def _fix_time_axis(self, features: torch.Tensor) -> torch.Tensor:
        current_frames = features.shape[-1]
        target_frames = 64
        if current_frames > target_frames:
            return features[..., :target_frames]
        if current_frames < target_frames:
            padding = target_frames - current_frames
            return nn.functional.pad(features, (0, padding))
        return features

    @staticmethod
    def _normalize(features: torch.Tensor) -> torch.Tensor:
        mean = features.mean()
        std = features.std().clamp_min(1e-6)
        return (features - mean) / std


class AudioClassifier(nn.Module):
    def __init__(self, num_classes: int) -> None:
        super().__init__()
        self.features = nn.Sequential(
            nn.Conv2d(1, 16, kernel_size=3, padding=1),
            nn.BatchNorm2d(16),
            nn.ReLU(),
            nn.MaxPool2d(2),
            nn.Conv2d(16, 32, kernel_size=3, padding=1),
            nn.BatchNorm2d(32),
            nn.ReLU(),
            nn.MaxPool2d(2),
            nn.Conv2d(32, 64, kernel_size=3, padding=1),
            nn.BatchNorm2d(64),
            nn.ReLU(),
            nn.AdaptiveAvgPool2d((1, 1)),
        )
        self.classifier = nn.Linear(64, num_classes)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = self.features(x)
        x = x.flatten(1)
        return self.classifier(x)


def make_loaders(dataset: Dataset, batch_size: int, seed: int) -> tuple[DataLoader, DataLoader]:
    train_size = max(1, math.floor(len(dataset) * 0.8))
    val_size = max(1, len(dataset) - train_size)
    if train_size + val_size > len(dataset):
        train_size = len(dataset) - 1
        val_size = 1
    generator = torch.Generator().manual_seed(seed)
    train_set, val_set = random_split(dataset, [train_size, val_size], generator=generator)
    train_loader = DataLoader(train_set, batch_size=batch_size, shuffle=True)
    val_loader = DataLoader(val_set, batch_size=batch_size, shuffle=False)
    return train_loader, val_loader


def evaluate(model: nn.Module, loader: DataLoader, device: str) -> tuple[float, float]:
    model.eval()
    total_loss = 0.0
    total_correct = 0
    total_examples = 0
    criterion = nn.CrossEntropyLoss()

    with torch.no_grad():
        for features, targets in loader:
            features = features.to(device)
            targets = targets.to(device)
            logits = model(features)
            loss = criterion(logits, targets)
            total_loss += loss.item() * targets.size(0)
            total_correct += (logits.argmax(dim=1) == targets).sum().item()
            total_examples += targets.size(0)

    return total_loss / max(1, total_examples), total_correct / max(1, total_examples)


def train(args: argparse.Namespace) -> None:
    seed_everything(args.seed)
    args.output_dir.mkdir(parents=True, exist_ok=True)

    dataset = AudioDataset(
        data_dir=args.data_dir,
        sample_rate=args.sample_rate,
        duration=args.duration,
        n_mels=args.n_mels,
    )
    train_loader, val_loader = make_loaders(dataset, args.batch_size, args.seed)

    model = AudioClassifier(num_classes=len(CLASSES)).to(args.device)
    optimizer = torch.optim.Adam(model.parameters(), lr=args.learning_rate)
    criterion = nn.CrossEntropyLoss()

    best_accuracy = 0.0
    best_state = None

    for epoch in range(1, args.epochs + 1):
        model.train()
        running_loss = 0.0
        total_examples = 0

        for features, targets in train_loader:
            features = features.to(args.device)
            targets = targets.to(args.device)

            optimizer.zero_grad()
            logits = model(features)
            loss = criterion(logits, targets)
            loss.backward()
            optimizer.step()

            running_loss += loss.item() * targets.size(0)
            total_examples += targets.size(0)

        train_loss = running_loss / max(1, total_examples)
        val_loss, val_accuracy = evaluate(model, val_loader, args.device)

        print(
            f"epoch={epoch:02d} train_loss={train_loss:.4f} "
            f"val_loss={val_loss:.4f} val_acc={val_accuracy:.4f}"
        )

        if val_accuracy >= best_accuracy:
            best_accuracy = val_accuracy
            best_state = {k: v.detach().cpu() for k, v in model.state_dict().items()}

    if best_state is None:
        raise RuntimeError("Nao foi possivel obter um estado treinado.")

    model.load_state_dict(best_state)

    checkpoint_path = args.output_dir / "model.pt"
    labels_path = args.output_dir / "labels.json"
    onnx_path = args.output_dir / "model.onnx"

    torch.save(
        {
            "state_dict": model.state_dict(),
            "classes": CLASSES,
            "sample_rate": args.sample_rate,
            "n_mels": args.n_mels,
            "frames": 64,
        },
        checkpoint_path,
    )

    labels_path.write_text(json.dumps(CLASSES, ensure_ascii=True, indent=2))
    export_onnx(model, onnx_path)

    print(f"checkpoint salvo em {checkpoint_path}")
    print(f"labels salvos em {labels_path}")
    print(f"onnx salvo em {onnx_path}")
    print("copie o arquivo model.onnx para backend-api/model.onnx")


def export_onnx(model: nn.Module, output_path: Path) -> None:
    model.eval()
    dummy = torch.randn(1, 1, 64, 64)
    torch.onnx.export(
        model.cpu(),
        dummy,
        output_path,
        input_names=["input"],
        output_names=["logits"],
        dynamic_axes={"input": {0: "batch"}, "logits": {0: "batch"}},
        opset_version=17,
    )


if __name__ == "__main__":
    train(parse_args())

