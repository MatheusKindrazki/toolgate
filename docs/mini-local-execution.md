# Execução autônoma de código local no Mini

Este documento descreve como o Mac Mini executa trabalho de código de forma autônoma
para este repositório. Ele trata do processo de desenvolvimento, não do produto.

## Fluxo

1. **Card Kanban:** o trabalho nasce como um card do Hermes, ligado a um Work-Control-ID.
2. **Worktree isolada:** o executor cria uma worktree própria, em branch nomeada, sem tocar
   no working tree principal do clone.
3. **Claude Code via gateway:** o Claude Code roda pelo gateway de tokens da frente,
   usando `kanban-claude-gateway`. Não há fallback de provider nem rotação de contas.
4. **Testes:** `cargo test --workspace` e `make check` (fmt, clippy com `-D warnings`,
   testes Rust e `swift build` em `Packages/App`) rodam antes do commit.
5. **Commit:** apenas os arquivos do escopo autorizado entram no commit.
6. **Push e PR draft:** push da branch nomeada e `gh pr create --draft`. Sem merge nem deploy.

## XIRP no MacBook

O XIRP no MacBook é opcional. A execução autônoma local no Mini não depende dele.

## Regras de operação

- **Um writer por Work:** só um executor escreve em um Work por vez; sem concorrência na branch.
- **Preservação de checkpoint:** mudanças concorrentes ou WIP existentes são preservadas,
  nunca descartadas; o trabalho continua na mesma branch e no mesmo card.
- **Evidência real:** saída e exit code dos comandos são guardados e reportados como
  foram. Falha, toolchain ausente ou build lento são relatados como fato, sem inventar sucesso.
- **Limites de autorização:** o executor age só no escopo autorizado no card. Se faltar
  autorização ou credencial, ou houver permission denial, ele para e relata. O card é
  fechado pelo coordenador, não pelo executor.

## O que este fluxo não é

Este fluxo não é enforcement de adapters do produto Toolgate. Codex e Hermes continuam
`unsupported` até verificação independente; executar trabalho aqui não altera esse status.
