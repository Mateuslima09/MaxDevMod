//! Automated diagnostic report generator for MaxDevMod support.
//!
//! Inspects ETS2 and ATS installations, `westplay.dll`, `westplay.scs`, `plugin.log`, `app.log`,
//! `game.log.txt`, and `game.crash.txt`, runs automated checks for the most common failure causes
//! (missing/outdated DLL, OpenGL instead of DirectX 11, game crashes, missing mod, unrecognized
//! screen), saves a single `.txt` report in `Documents\WestPlay\logs\`, and opens Explorer on it.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReportResult {
    pub path: String,
    pub summary: String,
    pub issues: Vec<String>,
}

/// Generates a full diagnostic report `.txt`, opens Explorer selecting it, and returns the
/// file path, concise summary, and list of detected issues.
pub fn generate(live_summary: &str) -> Result<DiagnosticReportResult, String> {
    let mut out = String::with_capacity(32 * 1024);
    let mut issues: Vec<String> = Vec::new();

    let stamp = timestamp_compact();
    let _ = writeln!(out, "================================================================");
    let _ = writeln!(out, " MaxDevMod - Relatorio de Diagnostico / Support Report");
    let _ = writeln!(out, " Gerado em (UTC epoch): {stamp}");
    let _ = writeln!(out, " Versao do app: {}", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(out, "================================================================\n");

    let docs = dirs_documents();
    let westplay_dir = docs.as_ref().map(|d| d.join("WestPlay"));
    let logs_dir = westplay_dir
        .as_ref()
        .map(|d| d.join("logs"))
        .or_else(|| westplay_host::log::place().and_then(|p| p.parent().map(Path::to_path_buf)))
        .ok_or_else(|| "Nao foi possivel localizar a pasta de logs.".to_owned())?;
    let _ = fs::create_dir_all(&logs_dir);

    // 1. Check installed games & westplay.dll / westplay.scs
    let games = westplay_setup::game::find();
    let mut games_section = String::new();
    if games.is_empty() {
        issues.push("Nenhum jogo (ETS2 ou ATS) foi encontrado automaticamente na Steam.".into());
        let _ = writeln!(games_section, "Nenhum jogo encontrado automaticamente.");
    }
    let bundled_dll = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("westplay.dll")))
        .filter(|p| p.exists());
    let bundled_dll_len = bundled_dll.as_ref().and_then(|p| fs::metadata(p).ok().map(|m| m.len()));

    for game in &games {
        let name = if game.dir.to_string_lossy().to_lowercase().contains("euro") {
            "ETS2"
        } else {
            "ATS"
        };
        let _ = writeln!(games_section, "[{name}] Pasta: {}", game.dir.display());
        let _ = writeln!(games_section, "  Versao detectada: {:?}", game.version);

        let dll = game.dir.join("bin").join("win_x64").join("plugins").join("westplay.dll");
        match fs::metadata(&dll) {
            Ok(meta) => {
                let _ = writeln!(games_section, "  westplay.dll: OK ({} bytes)", meta.len());
                if let Some(expected) = bundled_dll_len
                    && meta.len() != expected
                {
                    issues.push(format!(
                        "{name}: westplay.dll no jogo ({} bytes) diferente da versao do app ({expected} bytes) — reinstale o plugin pelo app com o jogo fechado.",
                        meta.len()
                    ));
                }
            }
            Err(_) => {
                let _ = writeln!(games_section, "  westplay.dll: AUSENTE ({})", dll.display());
                issues.push(format!("{name}: plugin westplay.dll nao esta instalado em bin/win_x64/plugins/."));
            }
        }

        if let Some(wp) = &westplay_dir {
            let sub = if name == "ETS2" { wp.join("ets2") } else { wp.clone() };
            let scs = sub.join("westplay.scs");
            match fs::metadata(&scs) {
                Ok(meta) => {
                    let _ = writeln!(games_section, "  westplay.scs: OK ({} bytes)", meta.len());
                }
                Err(_) => {
                    let _ = writeln!(games_section, "  westplay.scs: AUSENTE ({})", scs.display());
                    issues.push(format!("{name}: mod westplay.scs nao encontrado em {}.", sub.display()));
                }
            }
            let mod_screens = sub.join("mod_screens.txt");
            if let Ok(text) = fs::read_to_string(&mod_screens) {
                let count = text.lines().filter(|l| !l.trim().is_empty()).count();
                let _ = writeln!(games_section, "  mod_screens.txt: {count} tela(s) de mod mapeadas");
            }
        }
    }

    // 2. Inspect ETS2 & ATS game.log.txt and game.crash.txt
    let mut game_logs_section = String::new();
    if let Some(docs_dir) = &docs {
        for (label, folder) in [
            ("ETS2", "Euro Truck Simulator 2"),
            ("ATS", "American Truck Simulator"),
        ] {
            let gdir = docs_dir.join(folder);
            if !gdir.exists() {
                continue;
            }
            let crash = gdir.join("game.crash.txt");
            if let Ok(text) = fs::read_to_string(&crash) {
                let tail = last_lines(&text, 35);
                if !tail.trim().is_empty() {
                    let _ = writeln!(game_logs_section, "--- {label} game.crash.txt (ultimas 35 linhas) ---");
                    let _ = writeln!(game_logs_section, "{tail}\n");
                    if tail.to_lowercase().contains("westplay.dll") {
                        issues.push(format!(
                            "{label}: game.crash.txt menciona westplay.dll (possivel conflito ou versao incompativel do jogo)."
                        ));
                    } else {
                        issues.push(format!("{label}: existe um game.crash.txt registrado recentemente."));
                    }
                }
            }

            let glog = gdir.join("game.log.txt");
            if let Ok(text) = fs::read_to_string(&glog) {
                let lower = text.to_lowercase();
                if lower.contains("[gl]") && !lower.contains("[dx11]") {
                    issues.push(format!(
                        "{label}: o jogo foi aberto em OpenGL em vez de DirectX 11 — o MaxDevMod exige DirectX 11."
                    ));
                }
                if !lower.contains("loading 'westplay'") {
                    issues.push(format!(
                        "{label}: o game.log.txt nao mostra 'loading westplay' — o jogo pode nao ter carregado a DLL."
                    ));
                }
                let interesting: Vec<&str> = text
                    .lines()
                    .filter(|l| {
                        let ll = l.to_lowercase();
                        ll.contains("westplay")
                            || ll.contains("<error>")
                            || ll.contains("[dx11]")
                            || ll.contains("[gl]")
                            || ll.contains("euro truck simulator 2 init")
                            || ll.contains("american truck simulator init")
                            || ll.contains("failed to load")
                    })
                    .collect();
                let start = interesting.len().saturating_sub(60);
                let _ = writeln!(game_logs_section, "--- {label} game.log.txt (erros e linhas do MaxDevMod) ---");
                for line in &interesting[start..] {
                    let _ = writeln!(game_logs_section, "{line}");
                }
                let _ = writeln!(game_logs_section);
            }
        }
    }

    // 3. Inspect plugin.log and plugin.anterior.log
    let mut plugin_log_section = String::new();
    for fname in ["plugin.log", "plugin.anterior.log"] {
        let p = logs_dir.join(fname);
        if let Ok(text) = fs::read_to_string(&p)
            && !text.trim().is_empty()
        {
            if fname == "plugin.log" && text.contains("screen textures in use: 0") && !text.contains("live GPS fallback") {
                issues.push(
                    "plugin.log indica 'screen textures in use: 0' — nenhuma tela do MaxDevMod foi detectada no caminhao atual."
                        .into(),
                );
            }
            let _ = writeln!(plugin_log_section, "--- {fname} (ultimas 100 linhas) ---");
            let _ = writeln!(plugin_log_section, "{}\n", last_lines(&text, 100));
        }
    }

    // 4. Inspect app.log
    let mut app_log_section = String::new();
    if let Some(app_log) = westplay_host::log::place()
        && let Ok(text) = fs::read_to_string(&app_log)
    {
        let _ = writeln!(app_log_section, "--- app.log ({}) (ultimas 100 linhas) ---", app_log.display());
        let _ = writeln!(app_log_section, "{}\n", last_lines(&text, 100));
    }

    // Assemble final report (summary first!)
    let _ = writeln!(out, "=== RESUMO AUTOMATICO DE VERIFICACAO ===");
    if issues.is_empty() {
        let _ = writeln!(out, "OK: Nenhum problema critico detectado automaticamente nos arquivos.");
    } else {
        for (i, issue) in issues.iter().enumerate() {
            let _ = writeln!(out, "{}. [ATENCAO] {issue}", i + 1);
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "=== ESTADO AO VIVO DO APP ===");
    let _ = writeln!(out, "{live_summary}\n");

    let _ = writeln!(out, "=== JOGOS E INSTALACAO ===");
    let _ = writeln!(out, "{games_section}");

    if !plugin_log_section.is_empty() {
        let _ = writeln!(out, "=== LOGS DO PLUGIN ===");
        let _ = writeln!(out, "{plugin_log_section}");
    }
    if !app_log_section.is_empty() {
        let _ = writeln!(out, "=== LOG DO APP ===");
        let _ = writeln!(out, "{app_log_section}");
    }
    if !game_logs_section.is_empty() {
        let _ = writeln!(out, "=== LOGS E CRASHES DO JOGO ===");
        let _ = writeln!(out, "{game_logs_section}");
    }

    let report_path = logs_dir.join(format!("MaxDevMod-Diagnostico-{stamp}.txt"));
    fs::write(&report_path, &out).map_err(|e| format!("Falha ao salvar relatorio: {e}"))?;

    // Open Windows Explorer highlighting the newly created report file.
    let _ = Command::new("explorer.exe")
        .arg(format!("/select,{}", report_path.display()))
        .spawn();

    let summary = if issues.is_empty() {
        format!(
            "MaxDevMod {} — Relatorio salvo em {}\nNenhum problema critico detectado automaticamente.\n\n{}",
            env!("CARGO_PKG_VERSION"),
            report_path.display(),
            live_summary
        )
    } else {
        let bullet_list = issues
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {s}", i + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "MaxDevMod {} — Relatorio salvo em {}\nProblemas detectados:\n{}\n\n{}",
            env!("CARGO_PKG_VERSION"),
            report_path.display(),
            bullet_list,
            live_summary
        )
    };

    Ok(DiagnosticReportResult {
        path: report_path.display().to_string(),
        summary,
        issues,
    })
}

fn dirs_documents() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("Documents"))
}

fn last_lines(text: &str, max: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max);
    lines[start..].join("\n")
}

fn timestamp_compact() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
