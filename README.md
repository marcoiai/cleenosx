# Cleenosx

Cleenosx is a macOS-only storage cleanup app for understanding where SSD space went, especially the confusing "System Data" bucket. 
It scans, explains, classifies, lets the user select files or whole directories, and, remove them only after strong (3) confirmations.

The project has three entry points that share the same Rust domain logic:

- A Tauri desktop app with a React + Tailwind UI.
- A guided terminal CLI.
- A generated macOS Recovery helper script for manual review workflows.

## Objective

Is simple: help the user find what is taking space, drill into the biggest blocks, warn what is safer or riskier to remove, select whole files and/or directories, and, remove them only after strong (3) confirmations.

Implemented scan areas include:

- APFS volumes and mounted filesystems.
- `/System/Volumes/Data` large-block usage.
- AssetsV2 and known MobileAsset classes.
- Developer tool storage such as Xcode, simulators, Android SDK, Homebrew, Rust, and container tools.
- Local Time Machine snapshot listing.
- Risk classification and visible scan logs.

## Requirements

- macOS. This app is not intended to support Windows or Linux.
- Rust stable toolchain.
- Node.js and `pnpm`.
- Tauri 2 prerequisites for macOS development.

## Install

```sh
pnpm install
```

## Run The Desktop App

```sh
pnpm tauri:dev
```

For frontend-only development:

```sh
pnpm dev
```

## Run The CLI

```sh
cargo run -p cleanerx-cli
```

The CLI starts a guided menu and uses the same read-only scanners as the desktop app.

## Build

```sh
pnpm build
pnpm tauri:build
```

For Mac App Store preparation, see [docs/APP_STORE.md](docs/APP_STORE.md).

## Test And Check

```sh
cargo test
cargo check
pnpm build
```

## Project Layout

```text
crates/cleanerx-core/   Shared scanner, classifier, model, and recovery logic
crates/cleanerx-cli/    Guided terminal interface
src-tauri/              Tauri shell and command bridge
src/                    React desktop UI
docs/                   Product and engineering documentation
```

## Safety Model

CleanerX treats macOS storage cleanup as a high-risk operation.

- Scans are safe to run.
- Removals require explicitly selected files/directories and confirmations.
- Rust `target` directories are valid cleanup candidates because Cargo can rebuild them.
- Whole volumes, whole `AssetsV2`, broad system paths, projects, and user documents are not cleanup targets.
- SIP or `restricted` paths are marked as read-only/system risks.
- macOS command failures become logs and partial results instead of crashes.

## Documentation

- [Context](docs/CONTEXT.md) explains the product problem, users, goals, and MVP boundaries.
- [Architecture](docs/ARCHITECTURE.md) explains the workspace, data flow, scanners, safety rules, and extension points.
- [Mac App Store](docs/APP_STORE.md) tracks Store-specific signing, sandbox, and upload work.
See en_US bellow pt_BR
# cleenosx

**Recuperador honesto de espaço em disco para macOS.**

Pague uma refeição. Use para sempre.

`cleenosx` é um utilitário para ajudar usuários de macOS a entender onde o espaço em disco está indo e remover com segurança arquivos que normalmente ficam escondidos, esquecidos ou difíceis de interpretar.

Ele nasceu de um problema real: o macOS dizendo que o disco estava cheio, “System Data” ocupando dezenas de gigabytes, ferramentas prometendo limpeza grátis e depois travando tudo atrás de paywall.

Aqui não tem truque.

## O utilitário honesto

Sem “demo” falsa.
Sem “grátis” que só descobre o problema e cobra para resolver.
Sem assustar o usuário com números inflados.
Sem apagar coisa crítica sem explicação.

O `cleenosx` escaneia, classifica e explica.

Antes de apagar qualquer coisa, você é avisado, entende o risco e confirma a ação. Em operações mais sensíveis, o aplicativo pede confirmação múltipla.

A ideia é simples:

> mostrar o que está ocupando espaço, explicar o que provavelmente pode ser removido e deixar a decisão final com você.

## O que ele faz

* Escaneia áreas comuns de acúmulo de arquivos no macOS.
* Ajuda a identificar caches, arquivos temporários, runtimes antigos, imagens de containers, máquinas virtuais e resíduos de ferramentas de desenvolvimento.
* Classifica itens por nível de segurança.
* Mostra descrições claras antes de qualquer remoção.
* Pede confirmação antes de deletar.
* Evita prometer milagres.
* Ajuda usuários comuns sem esconder detalhes técnicos de quem quiser entender.

## Classificação de segurança

O `cleenosx` não trata todo arquivo grande como lixo.

Os itens encontrados são classificados por risco aproximado:

### Tranquilo remover

Arquivos normalmente seguros para remoção, como caches descartáveis, temporários ou artefatos que podem ser recriados.

Mesmo assim, o app informa antes de apagar.

### Provavelmente removível

Itens que costumam ser seguros, mas podem afetar alguma ferramenta, ambiente de desenvolvimento ou configuração local.

Exemplo: caches de build, imagens antigas, runtimes não utilizados.

### Atenção

Arquivos grandes que podem ser removíveis, mas exigem contexto.

O app explica o que encontrou e por que você deve revisar antes de decidir.

### Precisa pesquisar

Itens que o app não considera seguro classificar automaticamente.

Nesse caso, ele não força a barra. Ele informa que é melhor investigar antes de remover.

## O que NÃO É

`cleenosx` não é um antivírus.

`cleenosx` não é uma ferramenta mágica que promete recuperar centenas de gigabytes em qualquer máquina.

`cleenosx` não é um “otimizador” que mexe em tudo sem explicar.

`cleenosx` não tenta enganar você com uma varredura grátis e depois cobrar para apagar.

`cleenosx` não deve ser usado para apagar arquivos do sistema sem entender o impacto.

`cleenosx` não substitui backup.

## Filosofia

A maioria dos limpadores de disco tenta parecer mais inteligente do que o usuário.

O `cleenosx` faz o contrário: ele tenta deixar o usuário mais informado.

O objetivo não é apagar o máximo possível.
O objetivo é recuperar espaço com clareza, segurança e consentimento.

## Por que existe

No macOS, especialmente em máquinas com SSD pequeno, é comum ver situações como:

* “System Data” ocupando dezenas de gigabytes.
* Atualização do macOS reclamando de falta de espaço.
* Docker, Podman, simuladores e caches consumindo muito disco.
* Arquivos grandes escondidos em `~/Library`, `/System/Volumes/Data`, `/private/var` e outras áreas pouco claras.
* Usuários sem saber o que pode ou não pode ser apagado.

O `cleenosx` nasceu para tornar esse processo menos obscuro.

## Exemplos de áreas analisadas

Dependendo da versão e das permissões concedidas, o `cleenosx` pode ajudar a investigar:

* Caches do usuário.
* Logs antigos.
* Arquivos temporários.
* Caches de desenvolvimento.
* Artefatos de build.
* Containers e imagens locais.
* Máquinas virtuais.
* Dados de Docker, Podman ou ferramentas similares.
* Simuladores e runtimes antigos.
* Pastas grandes dentro de `~/Library`.
* Áreas volumosas dentro de `/System/Volumes/Data`.

Nem tudo será apagável pelo app. Algumas áreas do macOS exigem permissão elevada, análise manual ou execução em modo específico.

## Segurança primeiro

O `cleenosx` foi pensado com uma regra central:

> detectar é uma coisa; deletar é outra.

Por isso:

* O scanner pode mostrar itens suspeitos ou grandes.
* A remoção exige confirmação explícita.
* Itens sensíveis recebem avisos mais fortes.
* O app evita apagar automaticamente aquilo que não entende.
* O usuário continua no controle.

## Pagamento justo

A proposta é simples:

**me pague uma refeição e use para sempre.**

Nada de assinatura abusiva.
Nada de cobrar de novo todo mês para limpar cache.
Nada de transformar desespero por espaço em disco em armadilha comercial.

Você paga pouco, recebe o utilitário completo e tem direito de uso permanente.

## Para quem é

`cleenosx` é para:

* Quem tem Mac com pouco armazenamento.
* Quem vê “System Data” gigante e não sabe por onde começar.
* Quem tentou limpadores “grátis” e descobriu que não eram grátis.
* Desenvolvedores que acumulam caches, builds, containers e simuladores.
* Usuários que querem apagar com segurança, não no chute.

## Para quem talvez não seja

Talvez não seja para você se:

* Você espera um botão mágico que apaga tudo sem perguntar.
* Você não quer ler nenhum aviso antes de remover arquivos.
* Você quer uma ferramenta que prometa resultados impossíveis.
* Você quer mexer em arquivos críticos do sistema sem backup.

## Status do projeto

`cleenosx` está em desenvolvimento.

A missão principal é entregar uma ferramenta pequena, clara e honesta para recuperar espaço em disco no macOS.

Mais recursos serão adicionados com cuidado, sempre respeitando a ideia principal: transparência antes de remoção.

## Aviso importante

Sempre tenha backup dos seus dados importantes.

Mesmo com classificações de segurança, nenhuma ferramenta consegue saber perfeitamente o contexto de todos os arquivos em todas as máquinas.

Use com atenção, leia os avisos e confirme apenas aquilo que você entende ou aceita remover.

## Licença

A definir.

## Nome

`cleenosx` vem da ideia de limpar o macOS sem enrolação.

Limpar. Explicar. Confirmar. Remover.

Sem teatro.

# cleenosx

**An honest disk space recovery utility for macOS.**

Buy me a meal. Use it forever.

`cleenosx` is a utility designed to help macOS users understand where their disk space is going and safely remove files that are usually hidden, forgotten, or hard to interpret.

It was born from a real problem: macOS saying the disk was full, “System Data” taking dozens of gigabytes, and cleanup tools pretending to be free only to lock the actual cleanup behind a paywall.

No tricks here.

## The honest utility

No fake “demo”.
No “free” scan that only finds the problem and then charges you to fix it.
No inflated numbers to scare the user.
No deleting critical files without explanation.

`cleenosx` scans, classifies, and explains.

Before deleting anything, you are warned, shown the risk, and asked to confirm the action. For more sensitive operations, the app asks for multiple confirmations.

The idea is simple:

> show what is taking space, explain what is probably safe to remove, and leave the final decision to you.

## What it does

* Scans common macOS storage accumulation areas.
* Helps identify caches, temporary files, old runtimes, container images, virtual machines, and development leftovers.
* Classifies items by safety level.
* Shows clear descriptions before any deletion.
* Requires confirmation before removing anything.
* Avoids promising miracles.
* Helps regular users without hiding technical details from those who want to understand more.

## Safety classification

`cleenosx` does not treat every large file as junk.

Detected items are classified by approximate risk:

### Safe to remove

Files that are usually safe to remove, such as disposable caches, temporary files, or artifacts that can be recreated.

Even then, the app tells you before deleting them.

### Probably removable

Items that are usually safe, but may affect a tool, development environment, or local configuration.

Examples: build caches, old images, unused runtimes.

### Attention required

Large files that may be removable, but need context.

The app explains what it found and why you should review it before deciding.

### Needs research

Items the app does not consider safe to classify automatically.

In this case, it does not pretend to know. It tells you that further investigation is recommended before removal.

## What it is NOT

`cleenosx` is not an antivirus.

`cleenosx` is not a magic tool that promises to recover hundreds of gigabytes on every machine.

`cleenosx` is not an “optimizer” that changes everything without explanation.

`cleenosx` does not trick you with a free scan and then charge you to delete files.

`cleenosx` should not be used to delete system files without understanding the impact.

`cleenosx` is not a replacement for backups.

## Philosophy

Most disk cleaners try to look smarter than the user.

`cleenosx` does the opposite: it tries to make the user better informed.

The goal is not to delete as much as possible.
The goal is to recover space with clarity, safety, and consent.

## Why it exists

On macOS, especially on machines with smaller SSDs, it is common to see situations like:

* “System Data” taking dozens of gigabytes.
* macOS updates complaining about lack of free space.
* Docker, Podman, simulators, and caches consuming a lot of storage.
* Large files hidden inside `~/Library`, `/System/Volumes/Data`, `/private/var`, and other unclear areas.
* Users not knowing what can or cannot be safely deleted.

`cleenosx` exists to make this process less obscure.

## Examples of areas analyzed

Depending on the version and granted permissions, `cleenosx` may help investigate:

* User caches.
* Old logs.
* Temporary files.
* Development caches.
* Build artifacts.
* Local containers and images.
* Virtual machines.
* Docker, Podman, or similar tool data.
* Old simulators and runtimes.
* Large folders inside `~/Library`.
* Heavy areas inside `/System/Volumes/Data`.

Not everything will be removable by the app. Some macOS areas require elevated permissions, manual review, or a specific execution mode.

## Safety first

`cleenosx` is built around one central rule:

> detecting is one thing; deleting is another.

That means:

* The scanner may show suspicious or large items.
* Removal requires explicit confirmation.
* Sensitive items receive stronger warnings.
* The app avoids automatically deleting anything it does not understand.
* The user stays in control.

## Fair payment

The proposal is simple:

**buy me a meal and use it forever.**

No abusive subscription.
No charging you every month to clear caches.
No turning disk space anxiety into a commercial trap.

You pay a small amount, get the full utility, and keep permanent usage rights.

## Who it is for

`cleenosx` is for:

* People with Macs that have limited storage.
* People seeing huge “System Data” usage and not knowing where to start.
* People who tried “free” cleaners and found out they were not really free.
* Developers accumulating caches, builds, containers, and simulators.
* Users who want to delete safely, not blindly.

## Who it may not be for

It may not be for you if:

* You expect a magic button that deletes everything without asking.
* You do not want to read any warning before removing files.
* You want a tool that promises impossible results.
* You want to touch critical system files without backups.

## Project status

`cleenosx` is under development.

The main mission is to deliver a small, clear, and honest utility to recover disk space on macOS.

More features will be added carefully, always respecting the core idea: transparency before removal.

## Important warning

Always keep backups of your important data.

Even with safety classifications, no tool can perfectly understand the context of every file on every machine.

Use with attention, read the warnings, and only confirm removals you understand or accept.

## License

To be defined.

## Name

`cleenosx` comes from the idea of cleaning macOS without deception.

Clean. Explain. Confirm. Remove.

No theater.
