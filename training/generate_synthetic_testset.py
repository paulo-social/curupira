from __future__ import annotations

import argparse
from pathlib import Path

from generate_synthetic_dataset import CLASSES, generate_class
import random


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Gera um conjunto sintetico separado para teste pos-treino."
    )
    parser.add_argument("--output-dir", type=Path, default=Path("training/test_data"))
    parser.add_argument("--samples-per-class", type=int, default=12)
    parser.add_argument("--seed", type=int, default=4242)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rng = random.Random(args.seed)

    for class_name in CLASSES:
        generate_class(class_name, args.samples_per_class, args.output_dir, rng)

    print(f"conjunto de teste sintetico gerado em {args.output_dir}")
    print(f"classes: {', '.join(CLASSES)}")
    print(f"amostras por classe: {args.samples_per_class}")


if __name__ == "__main__":
    main()

