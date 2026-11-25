## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- When graphify-out/graph.json exists, run `graphify query "<question>"` before your first grep, search or source-file read for a codebase question, and put this rule in every subagent prompt that explores code. Read raw files only for lines the graph or an earlier read already located. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- A committed graphify-out/graph.json is rewritten whole on every build, so a large diff to it in a pull request is expected. Do not flag it as churn or suggest skipping the graph rebuild or its hooks.

## Build e Release (Obrigatório)

Sempre que compilar e subir uma nova versão/release do **MaxDevMod**:
1. Se houve alteração no plugin (DLL do jogo), compile antes:
   ```powershell
   cargo build --release -p westplay-plugin
   ```
2. **NUNCA** compile o `westplay-app` em release com `cargo build --release -p westplay-app` puro (isso ativa `cfg(dev)` do Tauri e gera um `.exe` tentando conectar em `localhost:5173`, agora bloqueado por `compile_error!` em `app/src/main.rs`).
   - Para atualizar só o executável local (sem gerar instalador):
     ```powershell
     npx.cmd @tauri-apps/cli build --no-bundle --config app/tauri.release.conf.json
     ```
   - Para gerar o instalador NSIS completo, passe **SEMPRE** `--config app/tauri.release.conf.json` (nunca `tauri build --bundles nsis` puro):
     ```powershell
     npx.cmd @tauri-apps/cli build --config app/tauri.release.conf.json
     ```
     *(Ou `npm.cmd run build:release` na raiz do projeto.)*
     Isso garante que o instalador inclua o `westplay.dll`, os hooks (`app/installer/hooks.nsh`), o ícone de release e os arquivos de licença (~36,6 MB). Sem essa config, o `westplay.dll` e os hooks ficam de fora.
3. Publique a release sempre no repositório **`Mateuslima09/MaxDevMod`** (`gh release create ... -R Mateuslima09/MaxDevMod`), nunca no `upstream`.
