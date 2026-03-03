# FinderBit IPTV MVP

MVP web em Rust + Dioxus para demonstrar importacao de playlist M3U, login local, navegacao estilo catalogo, favoritos, recentes e reproducao HLS via `video` + `hls.js`.

## Estrutura

```text
.
|-- Cargo.toml
|-- Dioxus.toml
|-- README.md
`-- packages
    |-- app_web
    |   |-- Cargo.toml
    |   |-- index.html
    |   |-- assets
    |   |   `-- main.css
    |   `-- src
    |       |-- app.rs
    |       |-- main.rs
    |       |-- components
    |       |   |-- channel_list.rs
    |       |   |-- mod.rs
    |       |   |-- player.rs
    |       |   |-- sidebar_groups.rs
    |       |   `-- top_bar.rs
    |       `-- services
    |           |-- mod.rs
    |           `-- playlist_service.rs
    `-- core
        |-- Cargo.toml
        `-- src
            |-- lib.rs
            |-- m3u.rs
            |-- models.rs
            `-- storage.rs
```

## Pre-requisitos

- Rust toolchain estavel
- `wasm32-unknown-unknown` instalado
- Dioxus CLI (`dx`)

## Instalar dependencias

```bash
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli
```

## Rodar o app

Na raiz do repositorio:

```bash
dx serve --platform web
```

O `Dioxus.toml` aponta automaticamente para o subpackage `app_web`.

## Como usar

1. Entre com um perfil e uma senha local na tela inicial.
2. Clique em `Demo` para testar rapido.
3. Ou cole uma URL `.m3u` / `.m3u8` no topo e clique em `Carregar`.
4. Para servidor IPTV com autenticacao, informe o endereco do servidor, o `usuario` e a `senha`; o app monta automaticamente a URL `get.php`.
5. Ou clique em `Importar arquivo .m3u` e selecione um arquivo local.
6. Use o destaque, os filtros e a busca para navegar no catalogo.
7. Clique em um card para tocar no player.
8. Clique na estrela ou em `Adicionar a minha lista` para favoritar.

## Persistencia

- Ultima playlist parseada: salva em `localStorage`
- Favoritos: salvos em `localStorage`
- Recentes (ultimos 10): salvos em `localStorage`
- Perfil/sessao local: salvo em `localStorage`

## Observacoes

- A carga por URL depende de CORS do host da playlist.
- O player usa suporte nativo HLS quando o navegador oferece `application/vnd.apple.mpegurl`.
- Quando necessario, o fallback usa `hls.js` via CDN definido em `packages/app_web/index.html`.
- A playlist demo usa duas streams publicas de teste. Se alguma expirar, troque os URLs em `packages/app_web/src/services/playlist_service.rs`.
