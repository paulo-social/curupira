# Training

Pipeline de treino para gerar o `model.onnx` do Curupira.

## Estrutura esperada do dataset

```text
training/data/
  ambiente/
    a1.wav
    a2.wav
  chuva/
    c1.wav
  motosserra/
    m1.wav
  tiro/
    t1.wav
```

Cada pasta representa uma classe. O script aceita arquivos `.wav` mono ou estereo, com taxa de amostragem variada.

## Ambiente Python

Use Python `3.13.2`, igual ao arquivo `.python-version`.

```bash
python -m venv training/.venv
source training/.venv/bin/activate
pip install -r training/requirements.txt
```

## Dataset sintetico para teste

Se voce quer apenas validar o pipeline fim a fim, gere um conjunto artificial:

```bash
python training/generate_synthetic_dataset.py --samples-per-class 24
```

Isso cria:

```text
training/data/ambiente/*.wav
training/data/chuva/*.wav
training/data/motosserra/*.wav
training/data/tiro/*.wav
```

Esses audios sao artificiais e servem so para teste tecnico do fluxo de treino e inferencia.

## Conjunto separado para teste

Depois do treino, gere outro conjunto sintetico independente:

```bash
python training/generate_synthetic_testset.py --samples-per-class 12
```

Isso cria `training/test_data`, separado de `training/data`.

## Treino

```bash
python training/train.py --epochs 20
```

Artefatos gerados:

- `training/runs/model.pt`
- `training/runs/model.onnx`
- `training/runs/labels.json`

Depois copie o modelo exportado:

```bash
cp training/runs/model.onnx backend-api/model.onnx
```

## Avaliacao simples

```bash
python training/evaluate.py
```

Esse comando usa `training/runs/model.pt` e avalia em `training/test_data`.

## Observacoes

- O treino usa `MelSpectrogram` com `64` bandas Mel e fixa a saida em `64x64`, que bate com a expectativa atual do backend.
- O modelo e uma CNN pequena, pensada para prototipacao e TCC. Para melhorar resultado, o maior ganho costuma vir do dataset e do balanceamento entre classes.
- Se suas classes estiverem desbalanceadas, vale adicionar mais amostras das classes raras antes de sofisticar a rede.
