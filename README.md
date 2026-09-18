# QA Lab Launcher — V1

Launcher desktop Windows per distribuire tool QA tramite repository e GitHub Releases. Al primo avvio usa OAuth Device Flow; il token viene salvato in **Windows Credential Manager** e non nel browser o in file locali.

## Funzioni V1

- login GitHub OAuth Device Flow, senza PAT o client secret;
- catalogo derivato da tutte le repository visibili all'utente (incluse private, collaborazioni e organizzazioni);
- repository senza manifest ignorate dalla vista principale e conteggiate come “non configurate”;
- rilevamento automatico di `Not installed`, `Updated`, `Update available`;
- download autenticato della latest release, estrazione ZIP sicura, verifica SHA-256 opzionale;
- installazioni versionate in `%LOCALAPPDATA%\QALab\tools\<repo>\<tag>` (le vecchie versioni restano disponibili per un futuro rollback);
- registro locale in `%LOCALAPPDATA%\QALab\registry.json` e log in `%LOCALAPPDATA%\QALab\logs\launcher.log`.

## Prerequisiti di sviluppo

1. Windows 10/11, Node.js 20+ e Rust stable con target MSVC.
2. Microsoft C++ Build Tools e WebView2 (normalmente già incluso in Windows 11).
3. Una GitHub OAuth App con **Enable Device Flow** attivo.

In GitHub: **Settings → Developer settings → OAuth Apps → New OAuth App**. Homepage e callback possono essere URL HTTPS descrittivi (il Device Flow non usa il callback). Abilitare esplicitamente `Device Flow`. Serve soltanto il **Client ID**: non creare né inserire un client secret nel progetto.

## Avvio sviluppo

PowerShell:

```powershell
$env:GITHUB_CLIENT_ID = "Iv1_il_tuo_client_id"
npm install
npm run tauri dev
```

Il Client ID viene acquisito in compilazione; può anche essere letto dall'ambiente a runtime in sviluppo. Non è un segreto. Il token OAuth richiesto usa gli scope `repo read:user`, necessari per leggere manifest e release anche nelle repository private accessibili.

Per provare un installer compilato senza Client ID incorporato, registrarlo una volta come variabile utente e riaprire il launcher:

```powershell
[Environment]::SetEnvironmentVariable("GITHUB_CLIENT_ID", "Iv1_il_tuo_client_id", "User")
```

La lettura runtime è supportata anche dalla build release; la configurazione incorporata in compilazione resta la modalità consigliata per distribuire l'installer al team.

## Build installer Windows

```powershell
$env:GITHUB_CLIENT_ID = "Iv1_il_tuo_client_id"
npm install
npm run tauri build
```

L'installer NSIS per utente viene creato sotto `src-tauri\target\release\bundle\nsis`. Per una release aziendale è consigliata la firma del binario e dell'installer con il certificato code-signing dell'organizzazione.

## Rendere compatibile una repository

Inserire `launcher.json` nella root del branch predefinito:

```json
{
  "name": "Whitelisting Tool",
  "description": "Gestione whitelist dei device QA",
  "entrypoint": "WhitelistingTool.exe",
  "icon": "launcher/icon.png",
  "asset": {
    "pattern": "-win-x64.zip",
    "sha256_asset": "WhitelistingTool-win-x64.zip.sha256"
  }
}
```

Schema:

| Campo | Obbligatorio | Significato |
|---|---:|---|
| `name` | sì | Nome mostrato nel catalogo |
| `description` | no | Descrizione breve |
| `entrypoint` | sì | Percorso relativo dell'entrypoint dentro lo ZIP (`.exe`, `.cmd`, `.bat` o `.ps1`), ad es. `app/Tool.exe` |
| `icon` | no | Percorso relativo di un'icona PNG/JPG nel repository |
| `asset.pattern` | no | Suffisso case-insensitive dell'asset ZIP; default `-win-x64.zip` |
| `asset.sha256_asset` | no | Nome esatto dell'asset testuale contenente hash SHA-256 (formato hash o `hash filename`) |

Pubblicare poi una **GitHub Release non draft e non prerelease**, ad esempio tag `v1.4.0`, con un asset come:

```text
WhitelistingTool-win-x64.zip
WhitelistingTool-win-x64.zip.sha256   # opzionale ma consigliato
```

La selezione è deterministica: tra gli asset della `latest release`, il launcher prende il primo ZIP il cui nome termina con `asset.pattern`. Per evitare ambiguità, pubblicare un solo asset per quel suffisso. La versione remota è sempre il tag della release; il campo `version` non appartiene al manifest.

## Limiti intenzionali della V1

- solo pacchetti ZIP Windows x64;
- nessun rollback nella UI (le versioni precedenti vengono però conservate);
- aggiornamenti manuali tramite pulsante, senza servizio in background;
- massimo 2.000 repository per account (20 pagine GitHub da 100).

## Verifica

```powershell
npm run test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```
