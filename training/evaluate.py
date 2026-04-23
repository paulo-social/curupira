from __future__ import annotations

import argparse
from pathlib import Path

import torch

from train import AudioClassifier, AudioDataset, CLASSES


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Avalia o checkpoint treinado em um conjunto separado.")
    parser.add_argument("--data-dir", type=Path, default=Path("training/test_data"))
    parser.add_argument("--checkpoint", type=Path, default=Path("training/runs/model.pt"))
    parser.add_argument("--sample-rate", type=int, default=16_000)
    parser.add_argument("--duration", type=float, default=2.048)
    parser.add_argument("--n-mels", type=int, default=64)
    parser.add_argument("--device", default="cuda" if torch.cuda.is_available() else "cpu")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    payload = torch.load(args.checkpoint, map_location="cpu")

    model = AudioClassifier(num_classes=len(CLASSES))
    model.load_state_dict(payload["state_dict"])
    model.to(args.device)
    model.eval()

    dataset = AudioDataset(
        data_dir=args.data_dir,
        sample_rate=args.sample_rate,
        duration=args.duration,
        n_mels=args.n_mels,
    )

    correct = 0
    total = 0

    for features, target in dataset:
        with torch.no_grad():
            logits = model(features.unsqueeze(0).to(args.device))
            prediction = logits.argmax(dim=1).item()
        correct += int(prediction == target.item())
        total += 1

    accuracy = correct / max(1, total)
    print(f"accuracy={accuracy:.4f} ({correct}/{total})")


if __name__ == "__main__":
    main()

