# Curupira

Sistema de monitoramento ambiental com captura de áudio, inferência local via ONNX, persistência em SQLite e painel web para acompanhamento de alertas.

## Módulos

- `edge-sentinel`: cliente Rust que captura áudio do microfone ou envia amostras `.wav` em modo simulação.
- `backend-api`: API em Rust com `Axum`, inferência via `tract-onnx` e persistência em SQLite.
- `frontend-panel`: dashboard React + Vite + Tailwind para monitorar o backend e os alertas.
- `training`: scripts Python para gerar dataset sintético, treinar o modelo e exportar `model.onnx`.

## Pré-requisitos

- Rust instalado
- Node.js 20+
- Python 3.13
- `pip` e `venv`
- Docker e Docker Compose, se quiser subir tudo em containers

## Estrutura do repositório

```text
backend-api/
edge-sentinel/
frontend-panel/
training/
samples/
docker-compose.yml
```

## Como treinar o modelo

### 1. Gerar dataset sintético de treino

```bash
python3 training/generate_synthetic_dataset.py --samples-per-class 24
```

Isso cria:

```text
training/data/ambiente/*.wav
training/data/chuva/*.wav
training/data/motosserra/*.wav
training/data/tiro/*.wav
```

Esses áudios são artificiais e servem para validar o pipeline, não para treino real de produção.

### 2. Criar ambiente Python

```bash
python3 -m venv training/.venv
source training/.venv/bin/activate
pip install -r training/requirements.txt
```

### 3. Treinar e exportar para ONNX

```bash
python training/train.py --epochs 20
```

Artefatos gerados:

- `training/runs/model.pt`
- `training/runs/model.onnx`
- `training/runs/labels.json`

### 4. Copiar o modelo para o backend

```bash
cp training/runs/model.onnx backend-api/model.onnx
```

## Status do pipeline de IA

O repositório já possui o fluxo necessário para treinar um modelo e usá-lo no `backend-api`:

- gerar dataset em `training/data`
- treinar com `training/train.py`
- exportar `training/runs/model.onnx`
- copiar o modelo para `backend-api/model.onnx`
- executar inferência no backend via `tract-onnx`

Compatibilidade atual confirmada:

- o treino exporta um modelo ONNX com entrada no formato `1x1x64x64`
- o `backend-api` monta a entrada de inferência no mesmo formato `1x1x64x64`
- as classes esperadas no fluxo atual são `ambiente`, `chuva`, `motosserra` e `tiro`

Limitação técnica importante:

- o pré-processamento do treino e o pré-processamento da inferência no backend não são idênticos
- no treino, o Python usa `torchaudio.transforms.MelSpectrogram` e `AmplitudeToDB`
- no backend, o Rust usa uma implementação simplificada própria para gerar as features

Na prática, isso significa que o pipeline funciona para prototipação e validação do fluxo, mas a acurácia real pode cair porque o modelo é servido com features diferentes das usadas no treino.

## Como testar o modelo treinado

### 1. Gerar dataset sintético separado para teste

```bash
python training/generate_synthetic_testset.py --samples-per-class 12
```

Isso cria um conjunto independente em:

```text
training/test_data/ambiente/*.wav
training/test_data/chuva/*.wav
training/test_data/motosserra/*.wav
training/test_data/tiro/*.wav
```

### 2. Avaliar o checkpoint treinado

```bash
python training/evaluate.py
```

O script usa:

- `training/runs/model.pt`
- `training/test_data`

E imprime uma acurácia simples no terminal.

## Como executar as aplicações localmente

### Backend API

Em um terminal na raiz:

```bash
cargo run -p backend-api
```

O backend sobe em `http://localhost:8080`.

Variáveis de ambiente suportadas:

- `DATABASE_URL`: caminho do banco SQLite. Padrão: `sqlite://data/alerts.db`
- `MODEL_PATH`: caminho do arquivo ONNX. Padrão: `model.onnx`
- `SIMULATION`: habilita carga de dados simulados no startup quando estiver com valor `1`, `true`, `yes` ou `on`

Para visualizar o `frontend-panel` com dados de exemplo sem depender do `edge-sentinel`, você pode subir o backend em modo simulação:

```bash
SIMULATION=1 cargo run -p backend-api
```

Nesse modo:

- O backend popula o banco com alertas recentes de exemplo apenas se ele estiver vazio.
- Os dados simulados são inseridos uma única vez por banco.
- Se já existirem registros em `alerts`, nenhum alerta extra é adicionado.
- As rotas continuam as mesmas; o modo de simulação apenas preenche o histórico inicial.

Se você quiser forçar um novo conjunto de alertas simulados, remova o arquivo do banco atual ou aponte `DATABASE_URL` para outro banco vazio.

Rotas principais:

- `POST /analyze`
- `GET /alerts`

### Frontend

Em outro terminal:

```bash
cd frontend-panel
npm install
npm run dev
```

O frontend sobe em `http://localhost:5173`.

Variáveis de ambiente úteis:

- `VITE_API_URL`: URL base da API. Padrão local: `http://localhost:8080`

Exemplo:

```bash
cd frontend-panel
VITE_API_URL=http://localhost:8080 npm run dev
```

O painel consulta `GET /alerts` a cada 5 segundos. Se o backend estiver inacessível, a interface mostra estado offline. Se o backend estiver online mas o banco não tiver alertas, o painel continua funcional, mas sem itens no histórico.

### Edge Sentinel

Em outro terminal:

```bash
cargo run -p edge-sentinel
```

Isso tenta capturar áudio do microfone e enviar buffers WAV de 5 segundos para o backend.

### Edge Sentinel em modo simulação

Para testar sem microfone, coloque arquivos `.wav` em `samples/` e rode:

```bash
SIMULATION=1 cargo run -p edge-sentinel
```

Opcionalmente:

```bash
SIMULATION=1 SAMPLES_DIR=samples cargo run -p edge-sentinel
```

## Como testar o sistema fim a fim

### Fluxo recomendado

1. Gere e treine o modelo.
2. Copie `training/runs/model.onnx` para `backend-api/model.onnx`.
3. Suba o backend com `cargo run -p backend-api`.
4. Suba o frontend com `npm run dev` dentro de `frontend-panel`.
5. Rode o `edge-sentinel` em modo simulação ou modo microfone.
6. Abra `http://localhost:5173` e acompanhe o painel.

### Fluxo rápido para testar só o painel

Se a ideia for apenas validar o `frontend-panel` com dados visíveis, sem microfone e sem envio de áudios:

1. Suba o backend com `SIMULATION=1 cargo run -p backend-api`.
2. Suba o frontend com `npm run dev` dentro de `frontend-panel`.
3. Abra `http://localhost:5173`.

Isso já deve exibir:

- backend online no card de status
- histórico de alertas preenchido
- gráfico com ocorrências por hora
- banner de alerta crítico quando o evento mais recente tiver confiança acima de 80%

### Teste manual da API

Com o backend em execução, você pode enviar um `.wav` manualmente:

```bash
curl -X POST http://localhost:8080/analyze \
  -F "file=@training/test_data/tiro/tiro_000.wav"
```

Consultar histórico:

```bash
curl http://localhost:8080/alerts
```

## Como executar com Docker

```bash
docker compose up --build
```

Serviços:

- `backend-api` na porta `8080`
- `frontend-panel` na porta `5173`

## Observações

- O dataset atual é sintético, então os resultados servem para teste técnico do fluxo.
- Se `backend-api/model.onnx` não for um modelo real, o backend cai no fallback heurístico definido no código.
- O pipeline de treino e exportação para ONNX já existe e está integrado ao `backend-api`, mas o pré-processamento de inferência em Rust ainda não replica exatamente o pré-processamento usado no treino em Python.
- O modo `SIMULATION=1` do `backend-api` não altera o contrato da API; ele apenas garante que o `GET /alerts` tenha dados iniciais quando o banco estiver vazio.
- O modo `SIMULATION=1` do `backend-api` é independente do modo `SIMULATION=1` do `edge-sentinel`. Eles podem ser usados juntos ou separadamente.
- A validação com `cargo check` pode depender do estado do toolchain Rust da máquina.
