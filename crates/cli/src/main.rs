use std::fmt::Write as _;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::pipeline::{run_turn, Engine, TurnReport};
use llm::{LlmConfig, OllamaClient};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use world::{PersonaId, RunStatus, SuccessCondition, Verdict, World};

#[derive(Parser)]
#[command(
    name = "phishbowl",
    about = "Headless turn-pipeline harness for the Phishbowl engine."
)]
struct Args {
    /// Scenario package to load.
    #[arg(default_value = "scenarios/it-support-credential/scenario.ron")]
    scenario: PathBuf,
    /// Ollama base URL.
    #[arg(long, default_value = "http://localhost:11434")]
    ollama: String,
    /// Player config (model name lives here). Missing file falls back to defaults.
    #[arg(long, default_value = "phishbowl.toml")]
    config: PathBuf,
    /// Override the configured model. Any string; passed to Ollama verbatim.
    #[arg(long)]
    model: Option<String>,
    /// Skip Ollama entirely; drive the engine with deterministic fallbacks.
    #[arg(long)]
    offline: bool,
    /// Print the raw Analyst response each turn (to diagnose structured-output issues).
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut world = scenario::load(&args.scenario)
        .with_context(|| format!("loading scenario {}", args.scenario.display()))?;

    let model = args
        .model
        .unwrap_or_else(|| LlmConfig::load(&args.config).model);
    let client = OllamaClient::new(args.ollama.clone(), model);

    let mut out = tokio::io::stdout();
    let online = if args.offline {
        false
    } else {
        match client.health().await {
            Ok(()) => true,
            Err(e) => {
                say(&mut out, &format!("llm: {e}")).await?;
                say(
                    &mut out,
                    "continuing in offline mode (deterministic fallback).",
                )
                .await?;
                false
            }
        }
    };
    let engine = Engine {
        client,
        online,
        debug: args.debug,
    };

    let mut contact = objective_owner(&world)?;
    briefing(&mut out, &world, &engine, &contact).await?;

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        out.write_all(prompt(&world, &contact).as_bytes()).await?;
        out.flush().await?;
        let Some(line) = lines.next_line().await? else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(command) = line.strip_prefix('/') {
            match handle_command(&mut out, &world, &mut contact, command).await? {
                Flow::Continue => continue,
                Flow::Quit => break,
            }
        }

        let report = run_turn(&mut world, &engine, &contact, line).await;
        render_turn(&mut out, &world, &contact, &report).await?;
        if report.status != RunStatus::Active {
            render_outcome(&mut out, report.status).await?;
            break;
        }
    }
    Ok(())
}

enum Flow {
    Continue,
    Quit,
}

async fn handle_command(
    out: &mut tokio::io::Stdout,
    world: &World,
    contact: &mut PersonaId,
    command: &str,
) -> Result<Flow> {
    let mut parts = command.split_whitespace();
    let verb = parts.next().unwrap_or_default();
    match verb {
        "quit" | "q" => return Ok(Flow::Quit),
        "who" => roster(out, world).await?,
        "state" => state(out, world, contact).await?,
        "log" => log_tail(out, world).await?,
        "talk" => {
            let id = parts.next().unwrap_or_default();
            if world.persona(&PersonaId::from(id)).is_some() {
                *contact = PersonaId::from(id);
                say(out, &format!("Now talking to {id}.")).await?;
            } else {
                say(out, &format!("No such persona: {id}. Try /who.")).await?;
            }
        }
        "help" | "" => say(out, HELP).await?,
        other => say(out, &format!("Unknown command /{other}. Try /help.")).await?,
    }
    Ok(Flow::Continue)
}

const HELP: &str = "commands: /who  /talk <id>  /state  /log  /help  /quit\n\
                    anything else is sent as a message to the current contact.";

async fn briefing(
    out: &mut tokio::io::Stdout,
    world: &World,
    engine: &Engine,
    contact: &PersonaId,
) -> Result<()> {
    let mode = if engine.online {
        "online (Ollama)"
    } else {
        "offline (deterministic fallback)"
    };
    let model = engine.client.model();
    let pretext = world
        .player
        .pretext
        .as_ref()
        .map_or("none", |p| p.claimed_identity.as_str());
    let lines = format!(
        "== {org} ==\n{desc}\nturn budget: {budget} · mode: {mode} · model: {model}\nyour pretext: {pretext}\nopening contact: {contact}\n\n{HELP}\n",
        org = world.org.name,
        desc = world.objective.description,
        budget = world.objective.turn_budget,
    );
    say(out, &lines).await
}

async fn render_turn(
    out: &mut tokio::io::Stdout,
    world: &World,
    contact: &PersonaId,
    report: &TurnReport,
) -> Result<()> {
    let persona = world.persona(contact).expect("contact exists");
    let mut text = format!("\n{}: {}\n", report.speaker, report.reply);
    let _ = writeln!(
        text,
        "  [suspicion {} ({:+}) · trust {} ({:+}){}]",
        persona.state.suspicion.get(),
        report.suspicion_delta,
        persona.state.trust.get(),
        report.trust_delta,
        verdict_suffix(report.verdict),
    );
    for entry in &report.reasons {
        let _ = writeln!(text, "   - {:?}: {}", entry.rule, entry.cause);
    }
    if let Some(notice) = &report.notice {
        let _ = writeln!(text, "  (llm: {notice})");
    }
    if let Some(raw) = &report.analyst_raw {
        let _ = writeln!(text, "  [analyst raw] {raw}");
    }
    say(out, &text).await
}

fn verdict_suffix(verdict: Option<Verdict>) -> String {
    verdict.map_or_else(String::new, |v| format!(" · verdict: {v:?}"))
}

async fn render_outcome(out: &mut tokio::io::Stdout, status: RunStatus) -> Result<()> {
    let message = match status {
        RunStatus::Won => "OBJECTIVE COMPLETE — the protected value was disclosed.",
        RunStatus::Detected => "DETECTED — org-wide suspicion tripped the security response.",
        RunStatus::OutOfTurns => "OUT OF TURNS — the turn budget ran out.",
        RunStatus::Active => return Ok(()),
    };
    say(out, &format!("\n=== {message} ===")).await
}

async fn roster(out: &mut tokio::io::Stdout, world: &World) -> Result<()> {
    let mut text = String::from("cast:\n");
    for p in &world.personas {
        let _ = writeln!(
            text,
            "  {id:14} {name} — {title}, {dept} [suspicion {s} · trust {t}]",
            id = p.id.as_str(),
            name = p.name,
            title = p.title,
            dept = p.department,
            s = p.state.suspicion.get(),
            t = p.state.trust.get(),
        );
    }
    say(out, &text).await
}

async fn state(out: &mut tokio::io::Stdout, world: &World, contact: &PersonaId) -> Result<()> {
    let p = world.persona(contact).expect("contact exists");
    let text = format!(
        "{name} ({id}) — suspicion {s} · trust {t} · mood {mood:?}\norg suspicion: {org} · turn {clock}/{budget} · status {status:?}",
        name = p.name,
        id = p.id.as_str(),
        s = p.state.suspicion.get(),
        t = p.state.trust.get(),
        mood = p.state.mood,
        org = world.org_suspicion.get(),
        clock = world.clock,
        budget = world.objective.turn_budget,
        status = world.status,
    );
    say(out, &text).await
}

async fn log_tail(out: &mut tokio::io::Stdout, world: &World) -> Result<()> {
    let mut text = String::from("recent causal log:\n");
    for entry in world.log.entries.iter().rev().take(12).rev() {
        let _ = writeln!(
            text,
            "  t{turn} {persona}: {rule:?} ({weight:+}) {cause}",
            turn = entry.turn,
            persona = entry.persona.as_str(),
            rule = entry.entry.rule,
            weight = entry.entry.weight,
            cause = entry.entry.cause,
        );
    }
    say(out, &text).await
}

fn objective_owner(world: &World) -> Result<PersonaId> {
    let SuccessCondition::SecretDisclosed(id) = &world.objective.success;
    world
        .secret(id)
        .map(|s| s.owner.clone())
        .ok_or_else(|| anyhow!("objective secret {id} has no owner"))
}

fn prompt(world: &World, contact: &PersonaId) -> String {
    format!("[t{}→{}] > ", world.clock + 1, contact.as_str())
}

async fn say(out: &mut tokio::io::Stdout, text: &str) -> Result<()> {
    out.write_all(text.as_bytes()).await?;
    out.write_all(b"\n").await?;
    out.flush().await?;
    Ok(())
}
