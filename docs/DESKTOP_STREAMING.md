# Player IPTV Desktop – Listas grandes em streaming

Este documento descreve a arquitetura de **streaming** para listas M3U/M3U8 grandes (~150 MB) no app **desktop** (Dioxus + Tokio), e como testar.

## Visão geral

- **Nenhum** `response.text().await` nem leitura do arquivo inteiro em uma `String`.
- **Remoto:** `reqwest` com `bytes_stream()`; conversão de bytes em linhas de forma incremental.
- **Local:** `std::fs::File` + `BufReader` + `read_line()`.
- **Parser:** incremental; emite um `ChannelItem` assim que #EXTINF + URL estão completos (não acumula tudo antes).
- **UI:** recebe itens por `tokio::sync::mpsc`; progresso (bytes + itens); lista paginada (200 itens por página); busca com debounce (300 ms).

## Estrutura

```
packages/
  m3u/                    # Crate nativo (não WASM)
    src/
      model.rs            # ChannelItem, ExtInfMeta
      parser.rs           # M3uIncrementalParser (linha a linha)
      source.rs           # load_m3u_from_file, load_m3u_from_url (+ progress)
      lib.rs
  app_desktop/            # Binário Dioxus desktop
    src/
      main.rs             # Launch
      lib.rs              # UI: import file/URL, progresso, paginação, busca
```

## Uso público do crate `iptv_m3u`

- `load_m3u_from_file(path)` → `(Receiver<ChannelItem>, Receiver<Progress>)`
- `load_m3u_from_url(url, RequestOptions)` → `async` → `(Receiver<ChannelItem>, Receiver<Progress>)`
- `RequestOptions`: `user_agent`, `basic_auth`, `bearer_token`, `headers` (custom).

A UI consome os receivers em tarefas assíncronas e atualiza o estado (lista + progresso) sem bloquear a thread principal.

## Como testar

### Pré-requisitos

- Rust estável.
- Workspace com `packages/m3u` e `packages/app_desktop` (já configurado no `Cargo.toml` do workspace).

### Build e execução

```bash
cd /caminho/playerIptvRs
cargo run -p app_desktop
```

### Teste com arquivo local

1. Clique em **"Importar arquivo local"**.
2. Escolha um `.m3u` ou `.m3u8` (pode ser grande).
3. Verifique:
   - Mensagem de progresso (bytes lidos / itens processados) atualizando.
   - Primeira página (200 itens) aparecendo em pouco tempo, sem esperar o parse completo.
   - Navegação por **Anterior** / **Próximo**.
   - UI permanecendo responsiva durante a importação.

### Teste com URL remota (com auth)

1. No campo **URL**, use a URL completa da playlist (ex.: `http://servidor/get.php?username=USER&password=PASS&type=m3u_plus`).
2. Se o servidor exigir **Basic Auth** além da query:
   - Preencha **Basic Auth: usuário** e **Basic Auth: senha**.
3. Se o servidor exigir **Bearer token**:
   - Preencha **Bearer token**.
4. Clique em **"Carregar URL"**.
5. Verifique o mesmo que no teste com arquivo (progresso, primeira página rápida, paginação, UI responsiva).

### Teste de busca

1. Com uma lista carregada, digite no campo **"Buscar por título"**.
2. Após ~300 ms (debounce), a lista filtrada deve atualizar.
3. A paginação passa a considerar apenas os itens filtrados.

## Notas de performance

- **Arquivo:** uma thread bloqueante lê linha a linha e envia itens pelo canal; a UI não espera o fim do arquivo.
- **URL:** a stream de bytes é consumida em uma task Tokio; o parsing e o envio para a UI são feitos em background.
- **Memória:** a lista completa de `ChannelItem` fica em memória uma vez; não há duplicação do buffer de texto da playlist.
- **Renderização:** apenas 200 itens por página são renderizados no VirtualDOM; a lista total pode ter dezenas de milhares de itens sem travar.

## Dependências adicionadas

- **packages/m3u:** `tokio`, `reqwest` (com feature `stream`), `futures-util`, `serde`.
- **packages/app_desktop:** `dioxus`, `dioxus-desktop` (com `tokio_runtime`), `iptv_m3u`, `rfd` (file dialog), `tokio`.

Se faltar alguma crate ou versão, ajuste os `Cargo.toml` conforme as versões atuais compatíveis do ecossistema.
